//! Pure event contract: the envelope, idempotency tracking, handler selection,
//! and gating. No I/O — see the `courses_apps` crate for handler execution.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A non-empty event identifier, used as an idempotency key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EventId(String);

impl EventId {
    /// Parses a raw id, rejecting empty or whitespace-only input.
    pub fn parse(raw: impl Into<String>) -> Result<Self, crate::Error> {
        let raw = raw.into();
        if raw.trim().is_empty() {
            return Err(crate::Error::MalformedEvent("empty event id".into()));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for EventId {
    type Error = crate::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<EventId> for String {
    fn from(id: EventId) -> Self {
        id.0
    }
}

/// The one envelope used both inbound (`POST /events`) and outbound (SSE).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    #[serde(rename = "type")]
    pub kind: String,
    pub payload: String,
}

// ---------------------------------------------------------------------------
// Parsing (boundary)
// ---------------------------------------------------------------------------

/// Parses a raw JSON body into a validated `Event`.
pub fn parse_event(body: &str) -> Result<Event, crate::Error> {
    serde_json::from_str(body).map_err(|e| crate::Error::MalformedEvent(e.to_string()))
}

// ---------------------------------------------------------------------------
// Idempotency (pure; the shell owns the Mutex)
// ---------------------------------------------------------------------------

/// Whether an id had been seen before this observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    Fresh,
    Duplicate,
}

/// A bounded record of recently observed event ids. Oldest ids evict first.
///
/// `capacity` is the dedup-window size (intended for windows up to a few thousand;
/// `observe` is O(n) per call).
#[derive(Debug, Clone)]
pub struct RecentIds {
    order: VecDeque<EventId>,
    capacity: usize,
}

impl Default for RecentIds {
    fn default() -> Self {
        Self::with_capacity(1)
    }
}

impl RecentIds {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// Records `id`, returning the next state and whether it was already present.
    pub fn observe(mut self, id: &EventId) -> (Self, Seen) {
        if self.order.contains(id) {
            return (self, Seen::Duplicate);
        }
        if self.order.len() >= self.capacity {
            self.order.pop_front();
        }
        self.order.push_back(id.clone());
        (self, Seen::Fresh)
    }
}

// ---------------------------------------------------------------------------
// Dispatch + gating (pure)
// ---------------------------------------------------------------------------

/// The handlers the server knows. Unknown event types select nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerKind {
    CpuBurst,
    Counter,
}

/// Selects a handler for an event type. Unknown types return `None` (200 no-op).
pub fn select(kind: &str) -> Option<HandlerKind> {
    match kind {
        "cpu-burst" => Some(HandlerKind::CpuBurst),
        "counter" => Some(HandlerKind::Counter),
        _ => None,
    }
}

/// Which handler kinds require the unlock secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    /// No secret configured; everything runs.
    Open,
    /// A secret is configured and applies to all handler kinds.
    All(String),
    /// A secret is configured and applies only to the listed kinds.
    Some {
        secret: String,
        kinds: Vec<HandlerKind>,
    },
}

/// The gate decision for one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// Decides whether a handler may run, given the gate and the client-provided secret.
pub fn gate(kind: HandlerKind, cfg: &Gate, provided: Option<&str>) -> Decision {
    let required = match cfg {
        Gate::Open => return Decision::Allow,
        Gate::All(secret) => secret,
        Gate::Some { secret, kinds } => {
            if !kinds.contains(&kind) {
                return Decision::Allow;
            }
            secret
        }
    };
    match provided {
        Some(p) if crate::token_matches(required, p) => Decision::Allow,
        _ => Decision::Deny,
    }
}

// ---------------------------------------------------------------------------
// Handler payload configs (pure)
// ---------------------------------------------------------------------------

const MAX_BURST_SECONDS: u32 = 120;

/// CPU-burst intensity, clamped to a safe set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Intensity {
    Low,
    Medium,
    High,
}

/// Validated CPU-burst parameters. `seconds` is hard-capped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuBurstConfig {
    pub seconds: u32,
    pub intensity: Intensity,
}

/// Unvalidated deserialization target for [`CpuBurstConfig::parse`].
#[derive(Deserialize)]
struct CpuBurstRaw {
    seconds: u32,
    intensity: Intensity,
}

impl CpuBurstConfig {
    /// Parses and clamps a cpu-burst payload.
    pub fn parse(payload: &str) -> Result<Self, crate::Error> {
        let raw: CpuBurstRaw = serde_json::from_str(payload)
            .map_err(|e| crate::Error::MalformedEvent(e.to_string()))?;
        Ok(Self {
            seconds: raw.seconds.min(MAX_BURST_SECONDS),
            intensity: raw.intensity,
        })
    }
}

/// Returns whether a `/state` collection is publicly readable.
pub fn is_public_collection(whitelist: &[String], collection: &str) -> bool {
    whitelist.iter().any(|c| c == collection)
}

// ---------------------------------------------------------------------------
// Gate configuration parsing (pure)
// ---------------------------------------------------------------------------

/// The result of parsing gate configuration from raw env strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateConfig {
    pub gate: Gate,
    /// Kind names in `CB_APPS_GATED` that did not match a known handler. The
    /// shell logs these; parsing stays pure.
    pub unknown_kinds: Vec<String>,
}

/// Parses the gate from the optional secret and the optional `CB_APPS_GATED`
/// value ("all", or a comma-separated list of handler-kind names). No secret
/// yields `Gate::Open`. A secret with `gated` absent or "all" yields
/// `Gate::All`. A secret with a kind list yields `Gate::Some` over the known
/// kinds; unknown names are collected into `unknown_kinds`.
pub fn parse_gate(secret: Option<&str>, gated: Option<&str>) -> GateConfig {
    let Some(secret) = secret else {
        return GateConfig {
            gate: Gate::Open,
            unknown_kinds: Vec::new(),
        };
    };
    let Some(raw) = gated else {
        // Secret present, no explicit gated config → default to All (gate everything).
        return GateConfig {
            gate: Gate::All(secret.to_string()),
            unknown_kinds: Vec::new(),
        };
    };
    if raw.trim().eq_ignore_ascii_case("all") {
        return GateConfig {
            gate: Gate::All(secret.to_string()),
            unknown_kinds: Vec::new(),
        };
    }
    let mut kinds = Vec::new();
    let mut unknown_kinds = Vec::new();
    for entry in raw.split(',') {
        let name = entry.trim();
        if name.is_empty() {
            continue;
        }
        match name {
            "cpu-burst" => kinds.push(HandlerKind::CpuBurst),
            "counter" => kinds.push(HandlerKind::Counter),
            other => unknown_kinds.push(other.to_string()),
        }
    }
    GateConfig {
        gate: Gate::Some {
            secret: secret.to_string(),
            kinds,
        },
        unknown_kinds,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // A1 — EventId + Event serde
    #[test]
    fn event_id_parse_ok() {
        let id = EventId::parse("abc").unwrap();
        assert_eq!(id.as_str(), "abc");
    }

    #[test]
    fn event_id_parse_empty_err() {
        assert!(matches!(
            EventId::parse(""),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    #[test]
    fn event_id_parse_whitespace_err() {
        assert!(matches!(
            EventId::parse("   "),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    #[test]
    fn event_serde_round_trip() {
        let json = r#"{"id":"x","type":"cpu-burst","payload":"{}"}"#;
        let ev: Event = serde_json::from_str(json).unwrap();
        assert_eq!(ev.id.as_str(), "x");
        assert_eq!(ev.kind, "cpu-burst");
        let out = serde_json::to_string(&ev).unwrap();
        assert!(out.contains("\"type\""));
        assert!(!out.contains("\"kind\""));
    }

    // A2 — parse_event
    #[test]
    fn parse_event_valid() {
        let ev = parse_event(r#"{"id":"x","type":"cpu-burst","payload":"{}"}"#).unwrap();
        assert_eq!(ev.kind, "cpu-burst");
    }

    #[test]
    fn parse_event_missing_id() {
        assert!(matches!(
            parse_event(r#"{"type":"cpu-burst","payload":"{}"}"#),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    #[test]
    fn parse_event_empty_id() {
        assert!(matches!(
            parse_event(r#"{"id":"","type":"cpu-burst","payload":"{}"}"#),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    #[test]
    fn parse_event_garbage() {
        assert!(matches!(
            parse_event("not json"),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    // A3 — RecentIds
    #[test]
    fn recent_ids_first_observe_fresh() {
        let store = RecentIds::with_capacity(10);
        let id = EventId::parse("a").unwrap();
        let (_, seen) = store.observe(&id);
        assert_eq!(seen, Seen::Fresh);
    }

    #[test]
    fn recent_ids_second_observe_duplicate() {
        let store = RecentIds::with_capacity(10);
        let id = EventId::parse("a").unwrap();
        let (store, _) = store.observe(&id);
        let (_, seen) = store.observe(&id);
        assert_eq!(seen, Seen::Duplicate);
    }

    #[test]
    fn recent_ids_eviction_makes_fresh() {
        let store = RecentIds::with_capacity(2);
        let a = EventId::parse("a").unwrap();
        let b = EventId::parse("b").unwrap();
        let c = EventId::parse("c").unwrap();
        let (store, _) = store.observe(&a);
        let (store, _) = store.observe(&b);
        // "a" gets evicted
        let (store, _) = store.observe(&c);
        let (_, seen) = store.observe(&a);
        assert_eq!(seen, Seen::Fresh);
    }

    #[test]
    fn recent_ids_capacity_zero_coerces_to_one() {
        let store = RecentIds::with_capacity(0);
        assert_eq!(store.capacity, 1);
    }

    #[test]
    fn recent_ids_default_has_capacity_one() {
        let store = RecentIds::default();
        let a = EventId::parse("a").unwrap();
        let b = EventId::parse("b").unwrap();
        // First observation is fresh.
        let (store, seen) = store.observe(&a);
        assert_eq!(seen, Seen::Fresh);
        // Adding a second distinct id evicts the first (capacity == 1).
        let (store, seen_b) = store.observe(&b);
        assert_eq!(seen_b, Seen::Fresh);
        // Re-observing "a" is Fresh again because it was evicted.
        let (_, seen_a_again) = store.observe(&a);
        assert_eq!(seen_a_again, Seen::Fresh);
    }

    // A4 — select
    #[test]
    fn select_cpu_burst() {
        assert_eq!(select("cpu-burst"), Some(HandlerKind::CpuBurst));
    }

    #[test]
    fn select_counter() {
        assert_eq!(select("counter"), Some(HandlerKind::Counter));
    }

    #[test]
    fn select_unknown() {
        assert_eq!(select("nope"), None);
        assert_eq!(select(""), None);
    }

    // A5 — gate
    #[test]
    fn gate_open_always_allow() {
        assert_eq!(
            gate(HandlerKind::CpuBurst, &Gate::Open, None),
            Decision::Allow
        );
        assert_eq!(
            gate(HandlerKind::CpuBurst, &Gate::Open, Some("wrong")),
            Decision::Allow
        );
    }

    #[test]
    fn gate_all_matching_secret_allow() {
        let cfg = Gate::All("secret".into());
        assert_eq!(
            gate(HandlerKind::CpuBurst, &cfg, Some("secret")),
            Decision::Allow
        );
    }

    #[test]
    fn gate_all_wrong_secret_deny() {
        let cfg = Gate::All("secret".into());
        assert_eq!(
            gate(HandlerKind::CpuBurst, &cfg, Some("wrong")),
            Decision::Deny
        );
    }

    #[test]
    fn gate_all_missing_secret_deny() {
        let cfg = Gate::All("secret".into());
        assert_eq!(gate(HandlerKind::CpuBurst, &cfg, None), Decision::Deny);
    }

    #[test]
    fn gate_some_covering_kind_behaves_like_all() {
        let cfg = Gate::Some {
            secret: "s".into(),
            kinds: vec![HandlerKind::CpuBurst],
        };
        assert_eq!(
            gate(HandlerKind::CpuBurst, &cfg, Some("s")),
            Decision::Allow
        );
        assert_eq!(gate(HandlerKind::CpuBurst, &cfg, None), Decision::Deny);
    }

    #[test]
    fn gate_some_not_covering_kind_allows_without_secret() {
        let cfg = Gate::Some {
            secret: "s".into(),
            kinds: vec![HandlerKind::CpuBurst],
        };
        assert_eq!(gate(HandlerKind::Counter, &cfg, None), Decision::Allow);
    }

    // A6 — CpuBurstConfig
    #[test]
    fn cpu_burst_config_parse_valid() {
        let cfg = CpuBurstConfig::parse(r#"{"seconds":30,"intensity":"high"}"#).unwrap();
        assert_eq!(cfg.seconds, 30);
        assert_eq!(cfg.intensity, Intensity::High);
    }

    #[test]
    fn cpu_burst_config_clamps_seconds() {
        let cfg = CpuBurstConfig::parse(r#"{"seconds":999,"intensity":"low"}"#).unwrap();
        assert_eq!(cfg.seconds, 120);
    }

    #[test]
    fn cpu_burst_config_exactly_max_seconds() {
        let cfg = CpuBurstConfig::parse(r#"{"seconds":120,"intensity":"medium"}"#).unwrap();
        assert_eq!(cfg.seconds, 120);
    }

    #[test]
    fn cpu_burst_config_bad_intensity() {
        assert!(matches!(
            CpuBurstConfig::parse(r#"{"seconds":10,"intensity":"ultra"}"#),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    #[test]
    fn cpu_burst_config_missing_field() {
        assert!(matches!(
            CpuBurstConfig::parse(r#"{"seconds":10}"#),
            Err(crate::Error::MalformedEvent(_))
        ));
    }

    // A8 — parse_gate
    #[test]
    fn parse_gate_no_secret_is_open() {
        let cfg = parse_gate(None, None);
        assert_eq!(cfg.gate, Gate::Open);
        assert!(cfg.unknown_kinds.is_empty());
    }

    #[test]
    fn parse_gate_no_secret_ignores_gated() {
        let cfg = parse_gate(None, Some("all"));
        assert_eq!(cfg.gate, Gate::Open);
        assert!(cfg.unknown_kinds.is_empty());
    }

    #[test]
    fn parse_gate_secret_no_gated_is_all() {
        let cfg = parse_gate(Some("s3cr3t"), None);
        assert_eq!(cfg.gate, Gate::All("s3cr3t".into()));
        assert!(cfg.unknown_kinds.is_empty());
    }

    #[test]
    fn parse_gate_secret_gated_all_is_all() {
        let cfg = parse_gate(Some("s3cr3t"), Some("all"));
        assert_eq!(cfg.gate, Gate::All("s3cr3t".into()));
        assert!(cfg.unknown_kinds.is_empty());
    }

    #[test]
    fn parse_gate_secret_gated_all_case_insensitive() {
        let cfg = parse_gate(Some("s3cr3t"), Some("ALL"));
        assert_eq!(cfg.gate, Gate::All("s3cr3t".into()));
        assert!(cfg.unknown_kinds.is_empty());
    }

    #[test]
    fn parse_gate_secret_kind_list_both_known() {
        let cfg = parse_gate(Some("s3cr3t"), Some("cpu-burst,counter"));
        assert_eq!(
            cfg.gate,
            Gate::Some {
                secret: "s3cr3t".into(),
                kinds: vec![HandlerKind::CpuBurst, HandlerKind::Counter],
            }
        );
        assert!(cfg.unknown_kinds.is_empty());
    }

    #[test]
    fn parse_gate_secret_kind_list_with_unknown() {
        let cfg = parse_gate(Some("s3cr3t"), Some("cpu-burst,bogus"));
        assert_eq!(
            cfg.gate,
            Gate::Some {
                secret: "s3cr3t".into(),
                kinds: vec![HandlerKind::CpuBurst]
            }
        );
        assert_eq!(cfg.unknown_kinds, vec!["bogus".to_string()]);
    }

    #[test]
    fn parse_gate_trims_whitespace_and_ignores_empty_entries() {
        let cfg = parse_gate(Some("s3cr3t"), Some(" cpu-burst , , counter , "));
        assert_eq!(
            cfg.gate,
            Gate::Some {
                secret: "s3cr3t".into(),
                kinds: vec![HandlerKind::CpuBurst, HandlerKind::Counter],
            }
        );
        assert!(cfg.unknown_kinds.is_empty());
    }

    // A7 — is_public_collection
    #[test]
    fn is_public_collection_present() {
        let wl = vec!["metrics".to_string(), "logs".to_string()];
        assert!(is_public_collection(&wl, "metrics"));
    }

    #[test]
    fn is_public_collection_absent() {
        let wl = vec!["metrics".to_string()];
        assert!(!is_public_collection(&wl, "private"));
    }

    #[test]
    fn is_public_collection_empty_whitelist() {
        assert!(!is_public_collection(&[], "anything"));
    }
}
