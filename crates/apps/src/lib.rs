#![deny(clippy::unwrap_used, clippy::expect_used)]

//! I/O crate: handler execution, DynamoDB, CPU load generation.
//!
//! This crate is the imperative shell for the courses platform. It owns all
//! side-effecting handler logic (DynamoDB reads/writes, CPU burst workers) and
//! exposes a single `dispatch` entry point for the server.

mod error;
mod handlers;

pub use error::{Error, Result};

use std::sync::Arc;

use courses_core::{Event, EventId, Gate};
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Shared context
// ---------------------------------------------------------------------------

/// Resolved configuration and I/O handles shared by handlers.
#[derive(Clone)]
pub struct AppsCtx {
    pub bus: broadcast::Sender<Event>,
    pub dynamo: aws_sdk_dynamodb::Client,
    pub gate: Gate,
    pub table: String,
    pub public_collections: Arc<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Outcome
// ---------------------------------------------------------------------------

/// The result of dispatching one event; the server maps this to an HTTP status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Handler accepted and ran (or was spawned). Map to HTTP 202.
    Accepted,
    /// No handler registered for this event type. Map to HTTP 200 no-op.
    Unknown,
    /// Gate denied the request. Map to HTTP 403.
    Denied,
}

// ---------------------------------------------------------------------------
// Bus helper (shell-internal)
// ---------------------------------------------------------------------------

/// Builds an `app-status` Event and sends it on the bus.
///
/// Status ids use the scheme `status-{key}` and are not deduplicated on the
/// client side (each emission is intentionally observable).
fn emit_status(ctx: &AppsCtx, key: &str, payload: String) {
    match EventId::parse(format!("status-{key}")) {
        Ok(id) => {
            let _ = ctx.bus.send(Event {
                id,
                kind: "app-status".into(),
                payload,
            });
        }
        Err(e) => tracing::error!("bad status id: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Dispatch entry point
// ---------------------------------------------------------------------------

/// Dispatches one event to the appropriate handler.
///
/// Handler failures are logged but never propagate as errors — the caller
/// always receives `Outcome::Accepted` when the gate passes.
pub async fn dispatch(ctx: &AppsCtx, event: Event, provided: Option<&str>) -> Outcome {
    let Some(kind) = courses_core::select(&event.kind) else {
        tracing::debug!("no handler for type {}", event.kind);
        return Outcome::Unknown;
    };

    if courses_core::gate(kind, &ctx.gate, provided) == courses_core::Decision::Deny {
        return Outcome::Denied;
    }

    let result = match kind {
        courses_core::HandlerKind::CpuBurst => handlers::cpu_burst(ctx, &event.payload).await,
        courses_core::HandlerKind::Counter => handlers::counter(ctx, &event.payload).await,
    };

    if let Err(e) = result {
        tracing::error!("handler {:?} failed: {e}", kind);
    }

    Outcome::Accepted
}

/// Reads a single item from DynamoDB, gated on the public-collection whitelist.
///
/// Returns `None` if the collection is not public or the item does not exist.
pub async fn read_item(
    ctx: &AppsCtx,
    collection: &str,
    key: &str,
) -> Result<Option<serde_json::Value>> {
    handlers::read_item(ctx, collection, key).await
}
