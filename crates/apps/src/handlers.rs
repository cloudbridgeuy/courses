use std::time::{Duration, Instant};

use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::AppsCtx;
use crate::emit_status;
use courses_core::{CpuBurstConfig, Intensity, is_public_collection};

// ---------------------------------------------------------------------------
// cpu_burst
// ---------------------------------------------------------------------------

/// Runs CPU-intensive busy-loops for the requested duration and intensity.
///
/// Worker count per intensity:
///   Low    -> 1 worker
///   Medium -> half of available parallelism (min 1)
///   High   -> all available parallelism (min 1)
///
/// NOTE: workers run on `spawn_blocking` threads, so cancelling/dropping this
/// async future does NOT stop them — they run to the deadline.  The deadline
/// itself bounds the resource leak.
pub async fn cpu_burst(ctx: &AppsCtx, payload: &str) -> Result<()> {
    let cfg = CpuBurstConfig::parse(payload)?;

    let available = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let workers = match cfg.intensity {
        Intensity::Low => 1,
        Intensity::Medium => (available / 2).max(1),
        Intensity::High => available,
    };

    // Status payload: plain string describing what we're about to do.
    emit_status(
        ctx,
        "cpu-burst-start",
        format!("burning {} workers for {}s", workers, cfg.seconds),
    );

    let deadline = Instant::now() + Duration::from_secs(u64::from(cfg.seconds));
    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        handles.push(tokio::task::spawn_blocking(move || {
            let mut i: u64 = 0;
            while std::time::Instant::now() < deadline {
                i = std::hint::black_box(i.wrapping_add(1));
            }
        }));
    }

    for handle in handles {
        if let Err(e) = handle.await {
            tracing::error!("cpu_burst worker panicked: {e}");
        }
    }

    // Status payload: plain string signalling completion.
    emit_status(
        ctx,
        "cpu-burst-done",
        format!("finished {}s burst", cfg.seconds),
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// counter
// ---------------------------------------------------------------------------

/// Shell-local DTO for the counter event payload.
#[derive(Deserialize)]
struct CounterPayload {
    key: String,
}

/// Status payload emitted after a counter increment.
#[derive(Serialize)]
struct CounterStatus<'a> {
    key: &'a str,
    value: &'a str,
}

/// Increments a counter in DynamoDB and emits the new value as an app-status.
///
/// DynamoDB schema:
///   Table primary key: collection (S) + key (S)
///   The counter attribute is named `value` (N).
pub async fn counter(ctx: &AppsCtx, payload: &str) -> Result<()> {
    let dto: CounterPayload = serde_json::from_str(payload)
        .map_err(|e| courses_core::Error::MalformedEvent(e.to_string()))?;

    let resp = ctx
        .dynamo
        .update_item()
        .table_name(&ctx.table)
        .key("collection", AttributeValue::S("counters".into()))
        .key("key", AttributeValue::S(dto.key.clone()))
        .update_expression("ADD #v :one")
        .expression_attribute_names("#v", "value")
        .expression_attribute_values(":one", AttributeValue::N("1".into()))
        .return_values(aws_sdk_dynamodb::types::ReturnValue::UpdatedNew)
        .send()
        .await
        .map_err(|e| Error::Dynamo(e.to_string()))?;

    let new_value = resp
        .attributes()
        .and_then(|attrs| attrs.get("value"))
        .and_then(|av| av.as_n().ok())
        .cloned()
        .unwrap_or_else(|| {
            tracing::warn!(
                "counter UpdatedNew response lacked expected `value` attribute; \
                 returning \"?\" as fallback"
            );
            "?".into()
        });

    // Status payload: JSON {"key":"...", "value":"..."} (value is the raw DynamoDB N string).
    let status_payload = serde_json::to_string(&CounterStatus {
        key: &dto.key,
        value: &new_value,
    })
    .map_err(|e| courses_core::Error::MalformedEvent(e.to_string()))?;
    emit_status(ctx, "counter-updated", status_payload);

    Ok(())
}

// ---------------------------------------------------------------------------
// read_item
// ---------------------------------------------------------------------------

/// Returns a DynamoDB item as JSON, or `None` if the collection is not public
/// or the item does not exist.
///
/// Supported AttributeValue subset:
///   S  -> JSON string
///   N  -> JSON string (preserves DynamoDB precision; callers parse as needed)
///   BOOL -> JSON bool
///   NULL -> JSON null
///   Everything else is omitted from the output object.
pub async fn read_item(
    ctx: &AppsCtx,
    collection: &str,
    key: &str,
) -> Result<Option<Value>> {
    if !is_public_collection(&ctx.public_collections, collection) {
        return Ok(None);
    }

    let resp = ctx
        .dynamo
        .get_item()
        .table_name(&ctx.table)
        .key("collection", AttributeValue::S(collection.into()))
        .key("key", AttributeValue::S(key.into()))
        .send()
        .await
        .map_err(|e| Error::Dynamo(e.to_string()))?;

    let item = match resp.item() {
        None => return Ok(None),
        Some(m) => m,
    };

    let mut obj = serde_json::Map::new();
    for (attr_key, av) in item {
        let json_val = match av {
            AttributeValue::S(s) => Value::String(s.clone()),
            AttributeValue::N(n) => Value::String(n.clone()),
            AttributeValue::Bool(b) => Value::Bool(*b),
            AttributeValue::Null(_) => Value::Null,
            _ => continue, // unsupported attribute type; skip
        };
        obj.insert(attr_key.clone(), json_val);
    }

    Ok(Some(Value::Object(obj)))
}
