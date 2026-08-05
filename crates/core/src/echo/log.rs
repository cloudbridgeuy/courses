//! The one-line record the echo server writes for every request it serves.
//!
//! It is an access log, so it holds what an access log answers: what was asked,
//! what was answered, who asked, and how long it took. `client` and `peer` are
//! both there on purpose — behind a load balancer they differ, and the pair is
//! the shortest demonstration of why an application logs the forwarded address
//! instead of the socket it sees.

use super::{EchoRequest, client_ip, first_header, group_headers, split_forwarded_for};

/// Value written for a field the request does not carry.
const ABSENT: &str = "-";

/// Header the ALB stamps on every request it forwards. It ties this line to the
/// balancer's own access log, and to an X-Ray trace.
const TRACE_HEADER: &str = "x-amzn-trace-id";

/// Builds the log line for one served request.
///
/// Single line, and `key=value` past the first three fields, so `grep` and
/// CloudWatch Logs Insights both read it without a parser.
pub fn access_log_line(request: &EchoRequest, status: u16, duration_ms: u64) -> String {
    let headers = group_headers(&request.headers);
    let forwarded = split_forwarded_for(&first_header(&headers, "x-forwarded-for"));
    let client = client_ip(request.peer.as_deref(), forwarded.as_deref());
    format!(
        "{method} {uri} -> {status} client={client} peer={peer} host={host} bytes={bytes} ms={duration_ms} trace={trace}",
        method = field(Some(&request.method)),
        uri = field(Some(&request.uri)),
        client = field(client.as_ref()),
        peer = field(request.peer.as_ref()),
        host = field(first_header(&headers, "host").as_ref()),
        bytes = request.body.len(),
        trace = field(first_header(&headers, TRACE_HEADER).as_ref()),
    )
}

/// Writes a field, or [`ABSENT`].
///
/// Every value here comes from the request, so a client picks it. Two kinds of
/// character therefore never reach the log: whitespace, which would break the
/// `key=value` reading and, worse, split one record into several, and control
/// characters. An `ESC` in a header is enough to paint text, move the cursor,
/// or clear the screen of whoever reads the log in a terminal — the subscriber
/// turns its own colors off when stdout is not a terminal, but it does not, and
/// cannot, know what is inside a value it is handed. Hyper answers 400 to a
/// header that holds one, so today this is the second line of defence; it is
/// here because the core must hold for any caller, not only for that server.
fn field(value: Option<&String>) -> String {
    let Some(text) = value.map(|v| v.trim()) else {
        return ABSENT.to_owned();
    };
    let clean: String = text
        .chars()
        .map(|c| {
            if c.is_whitespace() || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    if clean.is_empty() {
        ABSENT.to_owned()
    } else {
        clean
    }
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::EchoRequest;
    use super::access_log_line;

    fn request() -> EchoRequest {
        EchoRequest {
            method: "GET".to_owned(),
            uri: "/eco/prueba?status=503".to_owned(),
            path: "/eco/prueba".to_owned(),
            query_string: Some("status=503".to_owned()),
            http_version: "HTTP/1.1".to_owned(),
            headers: vec![("Host".to_owned(), "eco.example.com".to_owned())],
            body: Vec::new(),
            peer: Some("10.0.1.5:41234".to_owned()),
            local: Some("10.0.1.23:8080".to_owned()),
            received_at_unix_secs: 1_735_689_600,
            received_at_nanos: 0,
        }
    }

    #[test]
    fn writes_one_line_for_a_served_request() {
        assert_eq!(
            access_log_line(&request(), 503, 6),
            "GET /eco/prueba?status=503 -> 503 client=10.0.1.5 peer=10.0.1.5:41234 \
             host=eco.example.com bytes=0 ms=6 trace=-"
        );
    }

    #[test]
    fn takes_the_client_from_the_forwarded_chain() {
        let mut req = request();
        req.headers.push((
            "X-Forwarded-For".to_owned(),
            "203.0.113.7, 10.0.1.5".to_owned(),
        ));
        let line = access_log_line(&req, 200, 12);
        assert!(line.contains("client=203.0.113.7"), "{line}");
        assert!(line.contains("peer=10.0.1.5:41234"), "{line}");
    }

    #[test]
    fn keeps_the_trace_identifier_when_the_balancer_sent_one() {
        let mut req = request();
        req.headers.push((
            "X-Amzn-Trace-Id".to_owned(),
            "Root=1-63441c4a-abcdef012345".to_owned(),
        ));
        assert!(
            access_log_line(&req, 200, 1).contains("trace=Root=1-63441c4a-abcdef012345"),
            "the trace id must survive whole"
        );
    }

    #[test]
    fn reports_the_body_size() {
        let mut req = request();
        req.method = "POST".to_owned();
        req.body = b"hola".to_vec();
        assert!(access_log_line(&req, 200, 3).contains("bytes=4"));
    }

    #[test]
    fn writes_a_dash_for_what_the_request_does_not_carry() {
        let line = access_log_line(&EchoRequest::default(), 200, 0);
        assert!(line.contains("client=-"), "{line}");
        assert!(line.contains("peer=-"), "{line}");
        assert!(line.contains("host=-"), "{line}");
        assert!(line.contains("trace=-"), "{line}");
    }

    #[test]
    fn strips_the_escape_codes_a_client_put_in_a_header() {
        let mut req = request();
        req.headers = vec![(
            "Host".to_owned(),
            "\u{1b}[31mrojo\u{1b}[0m\u{9b}2J".to_owned(),
        )];
        let line = access_log_line(&req, 200, 0);
        assert!(!line.contains('\u{1b}'), "an ESC must never reach the log");
        assert!(!line.contains('\u{9b}'), "neither must a one-byte CSI");
        assert!(line.contains("host=_[31mrojo_[0m_2J"), "{line}");
    }

    #[test]
    fn collapses_whitespace_a_client_put_in_a_header() {
        let mut req = request();
        req.headers = vec![("Host".to_owned(), "eco example\n.com".to_owned())];
        let line = access_log_line(&req, 200, 0);
        assert!(line.contains("host=eco_example_.com"), "{line}");
        assert_eq!(line.lines().count(), 1);
    }
}
