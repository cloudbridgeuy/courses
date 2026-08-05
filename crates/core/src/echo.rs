//! Pure rendering of an HTTP request into the JSON document the echo server
//! returns.
//!
//! The shell captures a request into [`EchoRequest`] and hands it here; every
//! decision about what the answer looks like — how a query string is decoded,
//! how a binary body is represented, how a timestamp is written, which status
//! code it carries — is made by the pure functions in this module.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

mod log;
mod status;
mod time;

pub use log::access_log_line;
pub use status::{
    DEFAULT_ECHO_STATUS, EchoStatus, MAX_ECHO_STATUS, MIN_ECHO_STATUS, STATUS_QUERY_KEY,
    echo_status,
};
pub use time::format_rfc3339_utc;

use status::status_json;

/// Static facts about the running echo server, fixed at startup.
#[derive(Debug, Clone, Default)]
pub struct EchoServer {
    /// Public DNS name this server answers to when it sits behind HTTPS.
    pub public_name: Option<String>,
    /// Port the process listens on.
    pub port: u16,
    /// Host name of the machine, or of the container, running the process.
    pub hostname: Option<String>,
    /// Where this task sits in the network, read once from the ECS task
    /// metadata endpoint. `None` outside ECS, and when the endpoint is silent.
    pub ecs: Option<EcsNetwork>,
}

/// The network identity of an ECS task, as the task metadata endpoint reports
/// it. Every field is optional because the endpoint answers with what the
/// launch type knows: a Fargate task in `awsvpc` mode fills all of it, and
/// other combinations fill less.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EcsNetwork {
    pub cluster: Option<String>,
    pub task_id: Option<String>,
    pub family: Option<String>,
    pub revision: Option<String>,
    pub launch_type: Option<String>,
    pub availability_zone: Option<String>,
    pub network_mode: Option<String>,
    pub private_ipv4: Option<String>,
    pub private_dns_name: Option<String>,
    pub mac_address: Option<String>,
    pub subnet_cidr: Option<String>,
    pub subnet_gateway: Option<String>,
    pub dns_servers: Vec<String>,
}

/// One captured request. Owned data only: the shell copies out of the
/// framework's types so nothing here depends on axum, or on hyper.
#[derive(Debug, Clone, Default)]
pub struct EchoRequest {
    pub method: String,
    pub uri: String,
    pub path: String,
    pub query_string: Option<String>,
    pub http_version: String,
    /// Header names and values in the order they arrived. Names repeat when a
    /// header is sent more than once, and that repetition is preserved.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Address of the peer socket. Behind a load balancer this is the balancer,
    /// not the original client — which is what `x-forwarded-for` is for.
    pub peer: Option<String>,
    /// Address the connection landed on. In `awsvpc` mode this is the private
    /// IP of the task's own elastic network interface.
    pub local: Option<String>,
    pub received_at_unix_secs: i64,
    pub received_at_nanos: u32,
}

/// Largest body echoed back in full. Past it the body is reported by size only,
/// so a large upload cannot be reflected into an equally large response.
pub const MAX_ECHOED_BODY: usize = 64 * 1024;

/// The complete answer to one request: the status code to write, and the body.
#[derive(Debug, Clone)]
pub struct EchoAnswer {
    pub status: u16,
    pub body: Value,
}

// --- Pure core -------------------------------------------------------------

/// Decides everything about the answer to one request, so the shell only writes
/// what it is given.
pub fn echo_answer(server: &EchoServer, request: &EchoRequest) -> EchoAnswer {
    let status = echo_status(request.query_string.as_deref());
    EchoAnswer {
        status: status.code(),
        body: echo_json(server, request, &status),
    }
}

/// Builds the complete JSON answer for one request.
pub fn echo_json(server: &EchoServer, request: &EchoRequest, status: &EchoStatus) -> Value {
    let headers = group_headers(&request.headers);
    let host = first_header(&headers, "host");

    json!({
        "received_at": format_rfc3339_utc(request.received_at_unix_secs, request.received_at_nanos),
        "server": server_json(server, host.as_deref()),
        "response": status_json(status),
        "network": network_json(server, request, &headers),
        "request": {
            "method": request.method,
            "http_version": request.http_version,
            "uri": request.uri,
            "path": request.path,
            "path_segments": path_segments(&request.path),
            "query_string": request.query_string,
            "query": query_json(request.query_string.as_deref()),
            "host": host,
            "header_count": request.headers.len(),
            "headers": headers_json(&headers),
        },
        "body": body_json(&request.body),
    })
}

/// The server's own identity, plus whether the request arrived under the name
/// it was configured with. A `false` here means the request reached the server
/// some other way — by the balancer's own DNS name, or by IP.
fn server_json(server: &EchoServer, host: Option<&str>) -> Value {
    let matched = match (server.public_name.as_deref(), host) {
        (Some(name), Some(host)) => Some(host_matches_name(host, name)),
        (Some(_), None) => Some(false),
        (None, _) => None,
    };
    json!({
        "public_name": server.public_name,
        "matched_public_name": matched,
        "port": server.port,
        "hostname": server.hostname,
    })
}

/// Where the request came from, and where it landed.
///
/// Three answers to "who is calling", and they differ on purpose: `peer` is the
/// socket the process sees, which behind a load balancer is the balancer;
/// `forwarded_for` is the chain of hops it crossed; and `client_ip` is the one
/// an application should log.
fn network_json(
    server: &EchoServer,
    request: &EchoRequest,
    headers: &BTreeMap<String, Vec<String>>,
) -> Value {
    let forwarded_for = split_forwarded_for(&first_header(headers, "x-forwarded-for"));
    json!({
        "local": socket_json(request.local.as_deref()),
        "peer": socket_json(request.peer.as_deref()),
        "client_ip": client_ip(request.peer.as_deref(), forwarded_for.as_deref()),
        "forwarded_for": forwarded_for,
        "forwarded_proto": first_header(headers, "x-forwarded-proto"),
        "forwarded_port": first_header(headers, "x-forwarded-port"),
        "ecs": ecs_json(server.ecs.as_ref()),
    })
}

/// Splits a socket address into its parts, so the address can be read without
/// stripping a port first.
fn socket_json(address: Option<&str>) -> Value {
    match address.map(split_host_port) {
        Some((host, port)) => json!({ "address": host, "port": port, "socket": address }),
        None => Value::Null,
    }
}

/// The address to treat as the caller: the first hop of `x-forwarded-for` when
/// a proxy set it, and otherwise the peer socket.
///
/// The first hop is the one a load balancer appends the client address in front
/// of. It is trustworthy only as far as the proxy chain is: a client can send
/// its own `x-forwarded-for`, and only a proxy that overwrites it removes that.
pub fn client_ip(peer: Option<&str>, forwarded_for: Option<&[String]>) -> Option<String> {
    match forwarded_for.and_then(<[String]>::first) {
        Some(first) => Some(first.clone()),
        None => peer.map(|peer| split_host_port(peer).0),
    }
}

/// Splits `host:port` into its parts. Handles the bracketed form that IPv6
/// socket addresses are written in, `[::1]:8080`.
pub fn split_host_port(address: &str) -> (String, Option<u16>) {
    if let Some(rest) = address.strip_prefix('[')
        && let Some((host, tail)) = rest.split_once(']')
    {
        let port = tail.strip_prefix(':').and_then(|p| p.parse().ok());
        return (host.to_owned(), port);
    }
    // More than one colon, and no brackets, is a bare IPv6 address: the last
    // group is part of the address, not a port.
    if address.matches(':').count() > 1 {
        return (address.to_owned(), None);
    }
    match address.rsplit_once(':') {
        Some((host, port)) => match port.parse() {
            Ok(port) => (host.to_owned(), Some(port)),
            Err(_) => (address.to_owned(), None),
        },
        None => (address.to_owned(), None),
    }
}

fn ecs_json(ecs: Option<&EcsNetwork>) -> Value {
    match ecs {
        Some(ecs) => json!({
            "cluster": ecs.cluster,
            "task_id": ecs.task_id,
            "family": ecs.family,
            "revision": ecs.revision,
            "launch_type": ecs.launch_type,
            "availability_zone": ecs.availability_zone,
            "network_mode": ecs.network_mode,
            "private_ipv4": ecs.private_ipv4,
            "private_dns_name": ecs.private_dns_name,
            "mac_address": ecs.mac_address,
            "subnet_cidr": ecs.subnet_cidr,
            "subnet_gateway": ecs.subnet_gateway,
            "dns_servers": ecs.dns_servers,
        }),
        None => Value::Null,
    }
}

/// Reads the answer of the ECS task metadata endpoint, `${ECS_CONTAINER_METADATA_URI_V4}/task`.
///
/// Best effort by design: a body that is not JSON gives `None`, and any field
/// the endpoint leaves out stays `None`. The echo server must answer even when
/// the metadata is unavailable, or has changed shape.
pub fn parse_ecs_task_metadata(raw: &str) -> Option<EcsNetwork> {
    let task: Value = serde_json::from_str(raw).ok()?;

    // The task endpoint lists every container; the network belongs to the task,
    // so the first container that reports one describes them all.
    let network = task
        .get("Containers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|container| container.get("Networks"))
        .filter_map(Value::as_array)
        .flatten()
        .next();

    Some(EcsNetwork {
        cluster: text_field(&task, "Cluster").map(|arn| last_segment(&arn)),
        task_id: text_field(&task, "TaskARN").map(|arn| last_segment(&arn)),
        family: text_field(&task, "Family"),
        revision: text_field(&task, "Revision"),
        launch_type: text_field(&task, "LaunchType"),
        availability_zone: text_field(&task, "AvailabilityZone"),
        network_mode: network.and_then(|n| text_field(n, "NetworkMode")),
        private_ipv4: network
            .and_then(|n| n.get("IPv4Addresses"))
            .and_then(Value::as_array)
            .and_then(|addresses| addresses.first())
            .and_then(Value::as_str)
            .map(str::to_owned),
        private_dns_name: network.and_then(|n| text_field(n, "PrivateDNSName")),
        mac_address: network.and_then(|n| text_field(n, "MACAddress")),
        subnet_cidr: network.and_then(|n| text_field(n, "IPv4SubnetCIDRBlock")),
        subnet_gateway: network.and_then(|n| text_field(n, "SubnetGatewayIpv4Address")),
        dns_servers: network
            .and_then(|n| n.get("DomainNameServers"))
            .and_then(Value::as_array)
            .map(|servers| {
                servers
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Reads a field as text. `Revision` arrives as a string in some versions of
/// the endpoint and as a number in others, so both are accepted.
fn text_field(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Takes the last `/`-separated segment, which turns an ARN into the plain name
/// of the cluster, or the id of the task.
fn last_segment(arn: &str) -> String {
    arn.rsplit('/').next().unwrap_or(arn).to_owned()
}

/// Compares a `Host` header against a configured name, ignoring case and any
/// `:port` suffix the client may have sent.
pub fn host_matches_name(host: &str, name: &str) -> bool {
    let bare = host.split(':').next().unwrap_or(host);
    bare.eq_ignore_ascii_case(name.trim())
}

/// Groups headers by lowercase name, keeping every value of a repeated header
/// in arrival order.
pub fn group_headers(headers: &[(String, String)]) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, value) in headers {
        grouped
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push(value.clone());
    }
    grouped
}

/// A grouped header as JSON: a bare string when it appeared once, an array when
/// it repeated, so the common case reads cleanly.
fn headers_json(grouped: &BTreeMap<String, Vec<String>>) -> Value {
    let mut map = Map::new();
    for (name, values) in grouped {
        map.insert(name.clone(), one_or_many(values));
    }
    Value::Object(map)
}

fn one_or_many(values: &[String]) -> Value {
    match values {
        [single] => Value::String(single.clone()),
        many => Value::Array(many.iter().map(|v| Value::String(v.clone())).collect()),
    }
}

fn first_header(grouped: &BTreeMap<String, Vec<String>>, name: &str) -> Option<String> {
    grouped.get(name).and_then(|v| v.first()).cloned()
}

/// Splits the comma-separated chain of `x-forwarded-for` into its hops.
pub fn split_forwarded_for(raw: &Option<String>) -> Option<Vec<String>> {
    let raw = raw.as_deref()?;
    let hops: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|hop| !hop.is_empty())
        .map(str::to_owned)
        .collect();
    (!hops.is_empty()).then_some(hops)
}

/// Splits a path into its non-empty segments.
pub fn path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(percent_decode)
        .collect()
}

fn query_json(query: Option<&str>) -> Value {
    let mut map = Map::new();
    for (key, values) in parse_query(query.unwrap_or_default()) {
        map.insert(key, one_or_many(&values));
    }
    Value::Object(map)
}

/// Parses a query string into decoded keys and values. A repeated key keeps all
/// of its values, and a key with no `=` maps to an empty string.
pub fn parse_query(query: &str) -> BTreeMap<String, Vec<String>> {
    let mut parsed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        parsed
            .entry(percent_decode(key))
            .or_default()
            .push(percent_decode(value));
    }
    parsed
}

/// Decodes `%XX` escapes, and `+` as a space. Bytes that do not form valid
/// UTF-8 become the replacement character rather than an error: an echo server
/// reports what arrived, it does not reject it.
pub fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => match hex_pair(bytes[i + 1], bytes[i + 2]) {
                Some(byte) => {
                    out.push(byte);
                    i += 3;
                }
                // Not a valid escape: keep the `%` as written.
                None => {
                    out.push(b'%');
                    i += 1;
                }
            },
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_pair(high: u8, low: u8) -> Option<u8> {
    Some(hex_digit(high)? << 4 | hex_digit(low)?)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Describes the body: its size, how it is represented, and the content itself.
///
/// The representation is chosen from the bytes, never from the declared content
/// type — a client that lies about `Content-Type` still gets an honest answer.
pub fn body_json(body: &[u8]) -> Value {
    if body.is_empty() {
        return json!({ "size_bytes": 0, "encoding": "empty", "content": Value::Null });
    }
    if body.len() > MAX_ECHOED_BODY {
        return json!({
            "size_bytes": body.len(),
            "encoding": "omitted",
            "content": Value::Null,
            "note": format!("body over {MAX_ECHOED_BODY} bytes is reported by size only"),
        });
    }
    match std::str::from_utf8(body) {
        Ok(text) => match serde_json::from_str::<Value>(text) {
            Ok(parsed) => json!({
                "size_bytes": body.len(),
                "encoding": "json",
                "content": parsed,
            }),
            Err(_) => json!({
                "size_bytes": body.len(),
                "encoding": "text",
                "content": text,
            }),
        },
        Err(_) => json!({
            "size_bytes": body.len(),
            "encoding": "base64",
            "content": base64_encode(body),
        }),
    }
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding, for bodies that are not text.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let triple = b0 << 16 | b1 << 8 | b2;
        for slot in 0..4 {
            if slot <= chunk.len() {
                let index = (triple >> (18 - 6 * slot)) & 0x3f;
                out.push(char::from(BASE64_ALPHABET[index as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{
        DEFAULT_ECHO_STATUS, EchoRequest, EchoServer, EcsNetwork, MAX_ECHOED_BODY, base64_encode,
        body_json, client_ip, echo_answer, group_headers, host_matches_name,
        parse_ecs_task_metadata, parse_query, path_segments, percent_decode, split_forwarded_for,
        split_host_port,
    };
    use serde_json::{Value, json};

    fn header(name: &str, value: &str) -> (String, String) {
        (name.to_owned(), value.to_owned())
    }

    #[test]
    fn decodes_percent_escapes_and_plus() {
        assert_eq!(percent_decode("hola+mundo"), "hola mundo");
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("%C3%B1"), "ñ");
    }

    #[test]
    fn keeps_a_broken_escape_as_written() {
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("50%zz"), "50%zz");
    }

    #[test]
    fn replaces_bytes_that_are_not_utf8() {
        assert_eq!(percent_decode("%FF"), "\u{fffd}");
    }

    #[test]
    fn parses_a_query_string() {
        let parsed = parse_query("x=1&y=hola%20mundo");
        assert_eq!(parsed.get("x"), Some(&vec!["1".to_owned()]));
        assert_eq!(parsed.get("y"), Some(&vec!["hola mundo".to_owned()]));
    }

    #[test]
    fn keeps_every_value_of_a_repeated_key() {
        let parsed = parse_query("tag=a&tag=b");
        assert_eq!(
            parsed.get("tag"),
            Some(&vec!["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn maps_a_key_without_a_value_to_an_empty_string() {
        assert_eq!(parse_query("flag"), parse_query("flag="));
    }

    #[test]
    fn parses_an_empty_query_as_nothing() {
        assert!(parse_query("").is_empty());
        assert!(parse_query("&&").is_empty());
    }

    #[test]
    fn splits_path_segments_and_decodes_them() {
        assert_eq!(path_segments("/echo/a%20b/"), vec!["echo", "a b"]);
        assert_eq!(path_segments("/"), Vec::<String>::new());
    }

    #[test]
    fn groups_repeated_headers_in_arrival_order() {
        let grouped = group_headers(&[
            header("Accept", "text/html"),
            header("accept", "application/json"),
        ]);
        assert_eq!(
            grouped.get("accept"),
            Some(&vec!["text/html".to_owned(), "application/json".to_owned()])
        );
    }

    #[test]
    fn splits_the_forwarded_for_chain() {
        assert_eq!(
            split_forwarded_for(&Some("203.0.113.7, 10.0.1.5".to_owned())),
            Some(vec!["203.0.113.7".to_owned(), "10.0.1.5".to_owned()])
        );
        assert_eq!(split_forwarded_for(&None), None);
        assert_eq!(split_forwarded_for(&Some("  ".to_owned())), None);
    }

    #[test]
    fn matches_a_host_header_ignoring_case_and_port() {
        assert!(host_matches_name(
            "Echo.Example.Com:443",
            "echo.example.com"
        ));
        assert!(!host_matches_name("otro.example.com", "echo.example.com"));
    }

    #[test]
    fn encodes_base64_with_padding() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn reports_an_empty_body() {
        assert_eq!(body_json(b"")["encoding"], json!("empty"));
    }

    #[test]
    fn parses_a_json_body_instead_of_quoting_it() {
        let described = body_json(br#"{"a":1}"#);
        assert_eq!(described["encoding"], json!("json"));
        assert_eq!(described["content"], json!({"a": 1}));
    }

    #[test]
    fn reports_text_that_is_not_json_as_text() {
        let described = body_json(b"hola");
        assert_eq!(described["encoding"], json!("text"));
        assert_eq!(described["content"], json!("hola"));
        assert_eq!(described["size_bytes"], json!(4));
    }

    #[test]
    fn reports_bytes_that_are_not_text_as_base64() {
        let described = body_json(&[0xff, 0xfe]);
        assert_eq!(described["encoding"], json!("base64"));
        assert_eq!(described["content"], json!("//4="));
    }

    #[test]
    fn reports_an_oversized_body_by_size_only() {
        let big = vec![b'a'; MAX_ECHOED_BODY + 1];
        let described = body_json(&big);
        assert_eq!(described["encoding"], json!("omitted"));
        assert_eq!(described["content"], Value::Null);
        assert_eq!(described["size_bytes"], json!(MAX_ECHOED_BODY + 1));
    }

    fn sample() -> (EchoServer, EchoRequest) {
        let server = EchoServer {
            public_name: Some("echo.example.com".to_owned()),
            port: 8080,
            hostname: Some("ip-10-0-1-23".to_owned()),
            ecs: None,
        };
        let request = EchoRequest {
            method: "POST".to_owned(),
            uri: "/echo/a?x=1".to_owned(),
            path: "/echo/a".to_owned(),
            query_string: Some("x=1".to_owned()),
            http_version: "HTTP/1.1".to_owned(),
            headers: vec![
                header("Host", "echo.example.com"),
                header("X-Forwarded-For", "203.0.113.7"),
                header("X-Forwarded-Proto", "https"),
            ],
            body: b"hola".to_vec(),
            peer: Some("10.0.1.5:41234".to_owned()),
            local: Some("10.0.1.23:8080".to_owned()),
            received_at_unix_secs: 1_735_689_600,
            received_at_nanos: 0,
        };
        (server, request)
    }

    #[test]
    fn describes_a_whole_request() {
        let (server, request) = sample();
        let answer = echo_answer(&server, &request).body;

        assert_eq!(answer["received_at"], json!("2025-01-01T00:00:00.000Z"));
        assert_eq!(answer["request"]["method"], json!("POST"));
        assert_eq!(answer["request"]["path_segments"], json!(["echo", "a"]));
        assert_eq!(answer["request"]["query"]["x"], json!("1"));
        assert_eq!(answer["request"]["host"], json!("echo.example.com"));
        assert_eq!(answer["request"]["header_count"], json!(3));
        assert_eq!(
            answer["request"]["headers"]["x-forwarded-proto"],
            json!("https")
        );
        assert_eq!(answer["network"]["forwarded_for"], json!(["203.0.113.7"]));
        assert_eq!(answer["network"]["peer"]["address"], json!("10.0.1.5"));
        assert_eq!(answer["network"]["peer"]["port"], json!(41234));
        assert_eq!(answer["network"]["local"]["address"], json!("10.0.1.23"));
        assert_eq!(answer["network"]["client_ip"], json!("203.0.113.7"));
        assert_eq!(answer["network"]["ecs"], Value::Null);
        assert_eq!(answer["body"]["content"], json!("hola"));
        assert_eq!(answer["server"]["hostname"], json!("ip-10-0-1-23"));
    }

    #[test]
    fn reports_that_the_request_arrived_under_the_configured_name() {
        let (server, request) = sample();
        assert_eq!(
            echo_answer(&server, &request).body["server"]["matched_public_name"],
            json!(true)
        );
    }

    #[test]
    fn reports_a_request_that_arrived_by_another_name() {
        let (server, mut request) = sample();
        request.headers = vec![header("Host", "taller-alb-123.elb.amazonaws.com")];
        assert_eq!(
            echo_answer(&server, &request).body["server"]["matched_public_name"],
            json!(false)
        );
    }

    #[test]
    fn leaves_the_match_unknown_when_no_name_is_configured() {
        let (mut server, request) = sample();
        server.public_name = None;
        let answer = echo_answer(&server, &request).body;
        assert_eq!(answer["server"]["matched_public_name"], Value::Null);
        assert_eq!(answer["server"]["public_name"], Value::Null);
    }

    #[test]
    fn omits_nothing_when_the_request_is_bare() {
        let answer = echo_answer(&EchoServer::default(), &EchoRequest::default()).body;
        assert_eq!(answer["request"]["host"], Value::Null);
        assert_eq!(answer["request"]["query"], json!({}));
        assert_eq!(answer["network"]["forwarded_for"], Value::Null);
        assert_eq!(answer["network"]["peer"], Value::Null);
        assert_eq!(answer["network"]["client_ip"], Value::Null);
        assert_eq!(answer["body"]["encoding"], json!("empty"));
    }

    // --- Status code from the query string ---

    #[test]
    fn reports_a_requested_status_in_the_answer() {
        let (server, mut request) = sample();
        request.query_string = Some("status=503".to_owned());
        let answer = echo_answer(&server, &request);
        assert_eq!(answer.status, 503);
        assert_eq!(answer.body["response"]["status"], json!(503));
        assert_eq!(answer.body["response"]["source"], json!("query"));
        assert_eq!(answer.body["response"]["requested"], json!("503"));
    }

    #[test]
    fn reports_the_default_status_in_the_answer() {
        let (server, request) = sample();
        let answer = echo_answer(&server, &request);
        assert_eq!(answer.status, DEFAULT_ECHO_STATUS);
        assert_eq!(answer.body["response"]["source"], json!("default"));
        assert_eq!(answer.body["response"]["requested"], Value::Null);
    }

    #[test]
    fn answers_a_broken_status_with_the_default_and_an_explanation() {
        let (server, mut request) = sample();
        request.query_string = Some("status=teapot".to_owned());
        let answer = echo_answer(&server, &request);
        assert_eq!(answer.status, DEFAULT_ECHO_STATUS);
        assert_eq!(answer.body["response"]["requested"], json!("teapot"));
        assert!(
            answer.body["response"]["error"]
                .as_str()
                .is_some_and(|error| error.contains("status"))
        );
    }

    // --- Networking ---

    #[test]
    fn splits_an_ipv4_socket_address() {
        assert_eq!(
            split_host_port("10.0.1.5:41234"),
            ("10.0.1.5".to_owned(), Some(41234))
        );
    }

    #[test]
    fn splits_a_bracketed_ipv6_socket_address() {
        assert_eq!(
            split_host_port("[2001:db8::1]:443"),
            ("2001:db8::1".to_owned(), Some(443))
        );
    }

    #[test]
    fn leaves_an_address_without_a_port_whole() {
        assert_eq!(split_host_port("10.0.1.5"), ("10.0.1.5".to_owned(), None));
        assert_eq!(
            split_host_port("2001:db8::1"),
            ("2001:db8::1".to_owned(), None)
        );
    }

    #[test]
    fn takes_the_client_ip_from_the_first_forwarded_hop() {
        let chain = ["203.0.113.7".to_owned(), "10.0.1.5".to_owned()];
        assert_eq!(
            client_ip(Some("10.0.1.5:41234"), Some(&chain)),
            Some("203.0.113.7".to_owned())
        );
    }

    #[test]
    fn falls_back_to_the_peer_when_nothing_was_forwarded() {
        assert_eq!(
            client_ip(Some("203.0.113.7:41234"), None),
            Some("203.0.113.7".to_owned())
        );
        assert_eq!(client_ip(None, None), None);
    }

    /// Trimmed from the answer of `${ECS_CONTAINER_METADATA_URI_V4}/task` for a
    /// Fargate task in `awsvpc` mode.
    const TASK_METADATA: &str = r#"{
      "Cluster": "arn:aws:ecs:us-east-1:111122223333:cluster/taller-plataforma",
      "TaskARN": "arn:aws:ecs:us-east-1:111122223333:task/taller-plataforma/158d1c8083dd49d6b527399fd6414f5c",
      "Family": "taller-aws-maria-eco",
      "Revision": "3",
      "AvailabilityZone": "us-east-1b",
      "LaunchType": "FARGATE",
      "Containers": [
        {
          "Name": "app",
          "Networks": [
            {
              "NetworkMode": "awsvpc",
              "IPv4Addresses": ["10.0.2.100"],
              "MACAddress": "0e:9e:32:c7:48:85",
              "IPv4SubnetCIDRBlock": "10.0.2.0/24",
              "DomainNameServers": ["10.0.0.2"],
              "PrivateDNSName": "ip-10-0-2-100.ec2.internal",
              "SubnetGatewayIpv4Address": "10.0.2.1/24"
            }
          ]
        }
      ]
    }"#;

    #[test]
    fn reads_the_ecs_task_metadata() {
        let ecs = parse_ecs_task_metadata(TASK_METADATA).unwrap();
        assert_eq!(ecs.cluster.as_deref(), Some("taller-plataforma"));
        assert_eq!(
            ecs.task_id.as_deref(),
            Some("158d1c8083dd49d6b527399fd6414f5c")
        );
        assert_eq!(ecs.family.as_deref(), Some("taller-aws-maria-eco"));
        assert_eq!(ecs.revision.as_deref(), Some("3"));
        assert_eq!(ecs.availability_zone.as_deref(), Some("us-east-1b"));
        assert_eq!(ecs.launch_type.as_deref(), Some("FARGATE"));
        assert_eq!(ecs.network_mode.as_deref(), Some("awsvpc"));
        assert_eq!(ecs.private_ipv4.as_deref(), Some("10.0.2.100"));
        assert_eq!(
            ecs.private_dns_name.as_deref(),
            Some("ip-10-0-2-100.ec2.internal")
        );
        assert_eq!(ecs.mac_address.as_deref(), Some("0e:9e:32:c7:48:85"));
        assert_eq!(ecs.subnet_cidr.as_deref(), Some("10.0.2.0/24"));
        assert_eq!(ecs.subnet_gateway.as_deref(), Some("10.0.2.1/24"));
        assert_eq!(ecs.dns_servers, vec!["10.0.0.2".to_owned()]);
    }

    #[test]
    fn reads_a_revision_that_arrives_as_a_number() {
        let ecs = parse_ecs_task_metadata(r#"{"Revision": 12}"#).unwrap();
        assert_eq!(ecs.revision.as_deref(), Some("12"));
    }

    #[test]
    fn keeps_every_field_empty_when_the_metadata_says_nothing() {
        assert_eq!(parse_ecs_task_metadata("{}"), Some(EcsNetwork::default()));
    }

    #[test]
    fn refuses_metadata_that_is_not_json() {
        assert_eq!(parse_ecs_task_metadata("<html>404</html>"), None);
    }

    #[test]
    fn reports_the_task_network_in_the_answer() {
        let (mut server, request) = sample();
        server.ecs = parse_ecs_task_metadata(TASK_METADATA);
        let answer = echo_answer(&server, &request).body;
        assert_eq!(
            answer["network"]["ecs"]["availability_zone"],
            json!("us-east-1b")
        );
        assert_eq!(
            answer["network"]["ecs"]["subnet_cidr"],
            json!("10.0.2.0/24")
        );
        assert_eq!(answer["network"]["ecs"]["dns_servers"], json!(["10.0.0.2"]));
    }
}
