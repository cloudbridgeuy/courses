//! Imperative shell for the `echo` subcommand.
//!
//! Every request, whatever its method or path, is captured into a
//! [`EchoRequest`] and handed to `courses_core::echo_answer`, which decides what
//! the answer looks like, and which status code it carries. Nothing in this file
//! inspects a header, a query string, or a body: it only copies data out of
//! axum's types, reads the clock, and writes the result.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::connect_info::Connected;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::serve::IncomingStream;
use color_eyre::eyre::{Result, WrapErr};
use courses_core::{
    EchoRequest, EchoServer, EcsNetwork, access_log_line, echo_answer, parse_ecs_task_metadata,
};
use serde_json::json;
use tokio::net::TcpListener;

/// Largest request body read into memory. Bodies over this are refused with
/// 413; `courses_core::MAX_ECHOED_BODY` is the smaller size past which a body
/// is read but reported by size only.
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Environment variable the container runtime sets to the host name of the
/// task. On Fargate it identifies which task answered, which is what makes a
/// load balancing demonstration readable.
const HOSTNAME_ENV: &str = "HOSTNAME";

/// Environment variable ECS sets to the base URI of the task metadata endpoint.
/// Absent outside ECS.
const METADATA_URI_ENV: &str = "ECS_CONTAINER_METADATA_URI_V4";

/// How long to wait for the task metadata at startup. The endpoint is a local
/// link address, so a slow answer means it is not there.
const METADATA_TIMEOUT: Duration = Duration::from_secs(2);

pub struct EchoConfig {
    pub port: u16,
    pub public_name: Option<String>,
}

/// Both ends of the accepted connection.
///
/// The peer alone is not enough: in `awsvpc` mode the local address is the
/// private IP of the task's own network interface, which is the address the
/// target group registered.
#[derive(Clone, Debug)]
struct Sockets {
    peer: SocketAddr,
    local: Option<SocketAddr>,
}

impl Connected<IncomingStream<'_, TcpListener>> for Sockets {
    fn connect_info(stream: IncomingStream<'_, TcpListener>) -> Self {
        Self {
            peer: *stream.remote_addr(),
            local: stream.io().local_addr().ok(),
        }
    }
}

pub async fn serve(config: EchoConfig) -> Result<()> {
    let server = Arc::new(EchoServer {
        public_name: config.public_name.map(|name| name.trim().to_owned()),
        port: config.port,
        hostname: std::env::var(HOSTNAME_ENV).ok().filter(|h| !h.is_empty()),
        ecs: task_network().await,
    });

    match server.public_name.as_deref() {
        Some(name) => tracing::info!("echo server, published as https://{name}"),
        None => tracing::info!("echo server, with no public name configured"),
    }

    let router = Router::new().fallback(echo).with_state(Arc::clone(&server));

    // `ConnectInfo` needs the connection's addresses, and only this make service
    // carries them into the handler.
    axum::serve(
        crate::bind(config.port).await?,
        router.into_make_service_with_connect_info::<Sockets>(),
    )
    .with_graceful_shutdown(crate::shutdown_signal())
    .await
    .wrap_err("echo server error")
}

/// Reads the task's place in the network, once, at startup.
///
/// The metadata does not change while the task runs, so one call is enough, and
/// every failure is answered with `None`: the echo server must start outside
/// ECS, and must survive an endpoint that is slow or has changed shape.
async fn task_network() -> Option<EcsNetwork> {
    let base = std::env::var(METADATA_URI_ENV)
        .ok()
        .filter(|u| !u.is_empty());
    let Some(base) = base else {
        tracing::info!("{METADATA_URI_ENV} is not set: running outside ECS, no task metadata");
        return None;
    };

    let fetch = async {
        let client = reqwest::Client::builder()
            .timeout(METADATA_TIMEOUT)
            .build()?;
        client
            .get(format!("{}/task", base.trim_end_matches('/')))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    };

    match fetch.await {
        Ok(body) => {
            let parsed = parse_ecs_task_metadata(&body);
            if parsed.is_none() {
                tracing::warn!("the task metadata endpoint answered something that is not JSON");
            }
            parsed
        }
        Err(e) => {
            tracing::warn!("could not read the task metadata: {e}");
            None
        }
    }
}

async fn echo(
    State(server): State<Arc<EchoServer>>,
    ConnectInfo(sockets): ConnectInfo<Sockets>,
    request: Request,
) -> Response {
    let started = Instant::now();
    let (parts, body) = request.into_parts();

    let body = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(e) => {
            // The request never becomes an `EchoRequest`, so this is the one
            // answer the core does not write the log line for. Method and URI
            // are safe to print as they are: axum rejects a request target
            // that holds whitespace, or a control character.
            tracing::warn!("{} {} -> 413 rejected body: {e}", parts.method, parts.uri);
            return json_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                &json!({
                    "error": "request body too large",
                    "limit_bytes": MAX_BODY_BYTES,
                }),
            );
        }
    };

    let (received_at_unix_secs, received_at_nanos) = now();

    let captured = EchoRequest {
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
        path: parts.uri.path().to_owned(),
        query_string: parts.uri.query().map(str::to_owned),
        http_version: format!("{:?}", parts.version),
        headers: parts
            .headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect(),
        body: body.to_vec(),
        peer: Some(sockets.peer.to_string()),
        local: sockets.local.map(|addr| addr.to_string()),
        received_at_unix_secs,
        received_at_nanos,
    };

    let answer = echo_answer(&server, &captured);
    // One line per request, on stdout, which is where the container runtime
    // reads them from: `awslogs` sends them straight to the log group.
    tracing::info!(
        "{}",
        access_log_line(&captured, answer.status, elapsed_ms(started))
    );
    json_response(status_code(answer.status), &answer.body)
}

/// How long the request took, in whole milliseconds. Anything a request can
/// plausibly take fits; a saturating conversion keeps the handler panic-free.
fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Turns the code the core decided on into axum's type. The core keeps it in
/// range, so the fallback is unreachable, and still cannot panic.
fn status_code(code: u16) -> StatusCode {
    StatusCode::from_u16(code).unwrap_or(StatusCode::OK)
}

/// Reads the wall clock as seconds, and nanoseconds, since the Unix epoch.
fn now() -> (i64, u32) {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => (
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX),
            since.subsec_nanos(),
        ),
        // A clock set before 1970: report the negative offset rather than fail.
        Err(before) => (
            i64::try_from(before.duration().as_secs()).map_or(i64::MIN, |secs| -secs),
            0,
        ),
    }
}

/// Writes a value as indented JSON, so a plain `curl` is readable without a
/// formatter. Serialization of a `serde_json::Value` cannot fail, but the error
/// path still answers rather than panicking.
///
/// Every answer allows any cross-origin caller: the echo server holds nothing
/// private, and the in-guide `<cb-http>` client must be able to reach it from
/// a page served elsewhere (a local guide against the workshop ALB). The `*`
/// values also satisfy the preflight, which the fallback route answers like
/// any other request.
fn json_response(status: StatusCode, value: &serde_json::Value) -> Response {
    match serde_json::to_string_pretty(value) {
        Ok(body) => (
            status,
            [
                (header::CONTENT_TYPE, "application/json; charset=utf-8"),
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
                (header::ACCESS_CONTROL_ALLOW_METHODS, "*"),
                (header::ACCESS_CONTROL_ALLOW_HEADERS, "*"),
            ],
            format!("{body}\n"),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("failed to serialize the echo answer: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "serialization failed").into_response()
        }
    }
}
