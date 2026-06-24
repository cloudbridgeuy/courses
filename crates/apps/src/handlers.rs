use std::time::{Duration, Instant};

use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppsCtx;
use crate::emit_status;
use crate::error::{Error, Result};
use courses_core::{CpuBurstConfig, Intensity, MetricConfig, MetricMethod, is_public_collection};

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

    let available = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1);
    let workers = match cfg.intensity {
        Intensity::Low => 1,
        Intensity::Medium => (available / 2).max(1),
        Intensity::High => available,
    };

    tracing::info!(
        intensity = ?cfg.intensity,
        seconds = cfg.seconds,
        workers,
        available_cores = available,
        "cpu-burst started: generating CPU load (watch CloudWatch CPUUtilization)"
    );

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
            tracing::error!(error = %e, "cpu-burst worker panicked");
        }
    }

    tracing::info!(
        seconds = cfg.seconds,
        workers,
        "cpu-burst finished: CPU load released"
    );

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

    tracing::info!(
        key = %dto.key,
        table = %ctx.table,
        "counter increment requested: UpdateItem ADD on collection=counters"
    );

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
        .map_err(|e| {
            tracing::error!(key = %dto.key, table = %ctx.table, error = %e, "counter UpdateItem failed");
            Error::Dynamo(e.to_string())
        })?;

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

    tracing::info!(key = %dto.key, value = %new_value, "counter incremented");

    Ok(())
}

// ---------------------------------------------------------------------------
// metric
// ---------------------------------------------------------------------------

const METRIC_NAMESPACE: &str = "Taller/Custom";
const METRIC_NAME: &str = "CustomValue";
const METRIC_DIMENSION: &str = "method";

/// Publishes a custom metric to CloudWatch by the selected method.
///
/// EMF: writes one Embedded Metric Format log line to stdout; CloudWatch extracts
/// the metric from the container log group (no SDK, no extra IAM).
/// API: calls `PutMetricData` directly (needs `cloudwatch:PutMetricData`).
pub async fn metric(ctx: &AppsCtx, payload: &str) -> Result<()> {
    let cfg = MetricConfig::parse(payload)?;

    match cfg.method {
        MetricMethod::Emf => emit_emf(cfg.value),
        MetricMethod::Api => put_metric(ctx, cfg.value).await?,
    }

    // "emf"/"api" is an intentional English technical identifier (the method the
    // student selected), embedded in the otherwise-Spanish status text.
    let status_key = format!("metric-submitted-{}", cfg.method.as_str());
    emit_status(
        ctx,
        &status_key,
        format!(
            "CustomValue={} enviado ({})",
            cfg.value,
            cfg.method.as_str()
        ),
    );
    tracing::info!(
        value = cfg.value,
        method = cfg.method.as_str(),
        "custom metric submitted"
    );
    Ok(())
}

/// Writes one EMF log line. CloudWatch parses `_aws` and extracts `CustomValue`
/// with the `method=emf` dimension from whichever log group captures stdout.
fn emit_emf(value: i64) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .inspect_err(|e| tracing::warn!(error = %e, "system clock before Unix epoch; EMF timestamp will be 0"))
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = serde_json::json!({
        "_aws": {
            "Timestamp": ts,
            "CloudWatchMetrics": [{
                "Namespace": METRIC_NAMESPACE,
                "Dimensions": [[METRIC_DIMENSION]],
                "Metrics": [{ "Name": METRIC_NAME }]
            }]
        },
        METRIC_DIMENSION: "emf",
        METRIC_NAME: value,
    });
    // EMF requires a bare single-line JSON object on stdout; deliberately bypasses
    // the tracing envelope so CloudWatch's EMF parser sees the raw log line.
    println!("{line}");
    tracing::info!(value, "custom metric emitted via EMF log line");
}

/// Calls `PutMetricData` with the `method=api` dimension.
async fn put_metric(ctx: &AppsCtx, value: i64) -> Result<()> {
    use aws_sdk_cloudwatch::types::{Dimension, MetricDatum};

    let datum = MetricDatum::builder()
        .metric_name(METRIC_NAME)
        // safe: value is clamped to 0..=100 by MetricConfig::parse
        .value(value as f64)
        .dimensions(
            Dimension::builder()
                .name(METRIC_DIMENSION)
                .value("api")
                .build(),
        )
        .build();

    ctx.cloudwatch
        .put_metric_data()
        .namespace(METRIC_NAMESPACE)
        .metric_data(datum)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(value, error = %e, "PutMetricData failed");
            Error::CloudWatch(e.to_string())
        })?;

    tracing::info!(value, "custom metric submitted via PutMetricData");
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
pub async fn read_item(ctx: &AppsCtx, collection: &str, key: &str) -> Result<Option<Value>> {
    if !is_public_collection(&ctx.public_collections, collection) {
        tracing::warn!(
            collection,
            key,
            "rejected /state read: collection is not in the public allowlist"
        );
        return Ok(None);
    }

    tracing::debug!(collection, key, table = %ctx.table, "reading state item");

    let resp = ctx
        .dynamo
        .get_item()
        .table_name(&ctx.table)
        .key("collection", AttributeValue::S(collection.into()))
        .key("key", AttributeValue::S(key.into()))
        .send()
        .await
        .map_err(|e| {
            tracing::error!(collection, key, error = %e, "state GetItem failed");
            Error::Dynamo(e.to_string())
        })?;

    let Some(item) = resp.item() else {
        tracing::debug!(collection, key, "state item not found");
        return Ok(None);
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
