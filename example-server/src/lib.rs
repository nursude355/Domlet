#![no_std]

//! Allocation-free JSON-RPC and WebSocket helpers for the example servers.

use core::fmt::Write as _;
use heapless::String;
use serde::{Deserialize, Serialize};
use serde_json_core::str::EscapedStr;

/// Maximum incoming frame/message payload, excluding WebSocket framing bytes.
pub const RPC_MESSAGE_LIMIT: usize = 1024;

/// Apply the Pico's literal same-origin rule without allocating a URL.
///
/// Missing Origin is intentionally accepted for non-browser tools, as on the
/// desktop server. This is a browser-origin guard, not client authentication:
/// non-browser clients can omit or forge Origin. Present values must equal
/// `http://` followed by the nonempty Host, including its port and letter case.
pub fn websocket_origin_allowed(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let Some(host) = host.filter(|host| !host.is_empty()) else {
        return false;
    };
    origin.strip_prefix("http://") == Some(host)
}

/// Allocation-free heartbeat state, using monotonic milliseconds.
///
/// Probe immediately, then five seconds after each matching Pong. A Pong must
/// arrive before the probe's ten-second deadline. The lease also bounds a
/// stalled read/write before the next probe can be sent; ordinary RPC traffic,
/// peer Pings and unsolicited/stale Pongs cannot extend it.
#[derive(Clone, Copy, Debug)]
pub struct WebSocketHeartbeat {
    next_ping_ms: u64,
    deadline_ms: u64,
    sequence: u64,
    awaiting_pong: bool,
}

impl WebSocketHeartbeat {
    pub const PING_INTERVAL_MS: u64 = 5000;
    pub const PONG_TIMEOUT_MS: u64 = 10_000;

    pub fn new(now_ms: u64) -> Self {
        Self {
            next_ping_ms: now_ms,
            deadline_ms: now_ms.saturating_add(Self::PONG_TIMEOUT_MS),
            sequence: 0,
            awaiting_pong: false,
        }
    }

    pub fn deadline_ms(self) -> u64 {
        self.deadline_ms
    }

    /// Next time the I/O loop should wake to send a probe or expire the lease.
    pub fn wake_ms(self) -> u64 {
        if self.awaiting_pong {
            self.deadline_ms
        } else {
            self.next_ping_ms
        }
    }

    pub fn expired(self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }

    pub fn ping_if_due(&mut self, now_ms: u64) -> Option<[u8; 8]> {
        if self.expired(now_ms) || self.awaiting_pong || now_ms < self.next_ping_ms {
            return None;
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.awaiting_pong = true;
        // Do not move the deadline if a busy/stalled I/O loop sends late.
        Some(self.sequence.to_be_bytes())
    }

    pub fn accept_pong(&mut self, now_ms: u64, payload: &[u8]) -> bool {
        if self.expired(now_ms) || !self.awaiting_pong || payload != self.sequence.to_be_bytes() {
            return false;
        }
        self.awaiting_pong = false;
        self.next_ping_ms = now_ms.saturating_add(Self::PING_INTERVAL_MS);
        self.deadline_ms = self.next_ping_ms.saturating_add(Self::PONG_TIMEOUT_MS);
        true
    }
}

#[derive(Deserialize)]
struct Request<'a> {
    jsonrpc: &'a str,
    #[serde(default)]
    id: Option<u64>,
    method: &'a str,
    // Kept in its escaped JSON form: borrowing a plain `&str` fails for any
    // string containing escapes, and echoing it needs no unescape buffer.
    #[serde(default, borrow)]
    params: Option<EscapedStr<'a>>,
}

#[derive(Serialize)]
struct Success<'a, T> {
    jsonrpc: &'static str,
    id: Option<u64>,
    result: T,
    #[serde(skip)]
    _borrow: core::marker::PhantomData<&'a ()>,
}

#[derive(Serialize)]
struct CommandResult<'a> {
    status: &'static str,
    echo: Option<EscapedStr<'a>>,
}

#[derive(Serialize)]
struct Failure<'a> {
    jsonrpc: &'static str,
    id: Option<u64>,
    error: RpcError<'a>,
}

#[derive(Serialize)]
struct RpcError<'a> {
    code: i32,
    message: &'a str,
}

/// Parse one JSON-RPC request and create a response in a fixed-capacity buffer.
///
/// No allocator is used. A response with error code `-32603` is returned if the
/// caller-selected capacity is too small. Valid notifications (requests
/// without an `id`) are processed without a response, as JSON-RPC requires.
pub fn reply<const N: usize>(input: &str) -> Option<String<N>> {
    let Ok((request, _)) = serde_json_core::from_str::<Request<'_>>(input) else {
        return Some(serialize_or_internal(Failure {
            jsonrpc: "2.0",
            id: None,
            error: RpcError {
                code: -32700,
                message: "parse error",
            },
        }));
    };

    if request.jsonrpc != "2.0" {
        return Some(serialize_or_internal(Failure {
            jsonrpc: "2.0",
            id: request.id,
            error: RpcError {
                code: -32600,
                message: "invalid request",
            },
        }));
    }

    // Both methods are side-effect free, so a notification needs no dispatch.
    request.id?;
    Some(match request.method {
        "command" => serialize_or_internal(Success {
            jsonrpc: "2.0",
            id: request.id,
            result: CommandResult {
                status: "command accepted",
                echo: request.params,
            },
            _borrow: core::marker::PhantomData,
        }),
        "ping" => serialize_or_internal(Success {
            jsonrpc: "2.0",
            id: request.id,
            result: "pong",
            _borrow: core::marker::PhantomData,
        }),
        _ => serialize_or_internal(Failure {
            jsonrpc: "2.0",
            id: request.id,
            error: RpcError {
                code: -32601,
                message: "method not found",
            },
        }),
    })
}

/// Create a server-to-browser JSON-RPC notification without allocation.
pub fn telemetry<const N: usize>(uptime_seconds: u64) -> String<N> {
    let mut notification = String::new();
    let _ = write!(
        notification,
        "{{\"jsonrpc\":\"2.0\",\"method\":\"telemetry\",\"params\":{{\"uptime_s\":{uptime_seconds}}}}}"
    );
    notification
}

fn serialize_or_internal<const N: usize, T: Serialize>(value: T) -> String<N> {
    serde_json_core::to_string::<_, N>(&value).unwrap_or_else(|_| {
        let mut response = String::new();
        let _ = response.push_str(
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"response buffer too small"}}"#,
        );
        response
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_origin_matches_host_and_port_exactly() {
        for host in ["192.168.1.42", "device.local", "device.local:8080", "[::1]"] {
            let mut origin = String::<64>::try_from("http://").unwrap();
            origin.push_str(host).unwrap();
            assert!(websocket_origin_allowed(Some(&origin), Some(host)));
        }
        for origin in [
            "http://other.local",
            "http://device.local:8080",
            "http://device.local:80",
            "https://device.local",
            "HTTP://device.local",
            "http://DEVICE.local",
            "http://device.local/",
            "http://device.local/path",
            "null",
            "",
        ] {
            assert!(!websocket_origin_allowed(
                Some(origin),
                Some("device.local")
            ));
        }
        assert!(!websocket_origin_allowed(
            Some("http://device.local"),
            Some("DEVICE.local")
        ));
        assert!(!websocket_origin_allowed(
            Some("http://device.local"),
            Some("device.local:8080")
        ));
    }

    #[test]
    fn websocket_origin_missing_headers_and_empty_host() {
        assert!(websocket_origin_allowed(None, Some("device.local")));
        assert!(websocket_origin_allowed(None, None));
        assert!(!websocket_origin_allowed(Some("http://device.local"), None));
        assert!(!websocket_origin_allowed(Some("http://"), Some("")));
    }

    #[test]
    fn heartbeat_probes_immediately_and_keeps_idle_browser_alive() {
        let mut heartbeat = WebSocketHeartbeat::new(100);
        assert_eq!(heartbeat.wake_ms(), 100);
        let first = heartbeat.ping_if_due(100).unwrap();
        assert_eq!(heartbeat.deadline_ms(), 10_100);
        assert_eq!(heartbeat.wake_ms(), 10_100);
        assert_eq!(heartbeat.ping_if_due(101), None);
        assert!(heartbeat.accept_pong(200, &first));
        assert_eq!(heartbeat.deadline_ms(), 15_200);
        assert_eq!(heartbeat.wake_ms(), 5200);
        assert_eq!(heartbeat.ping_if_due(5199), None);
        let second = heartbeat.ping_if_due(5200).unwrap();
        assert_ne!(first, second);
        assert!(heartbeat.accept_pong(5300, &second));
        assert_eq!(heartbeat.deadline_ms(), 20_300);
        assert!(!heartbeat.expired(20_299));
        assert!(heartbeat.expired(20_300));
    }

    #[test]
    fn heartbeat_ignores_unsolicited_stale_wrong_and_late_pongs() {
        let mut heartbeat = WebSocketHeartbeat::new(0);
        assert!(!heartbeat.accept_pong(0, &0_u64.to_be_bytes()));
        let first = heartbeat.ping_if_due(0).unwrap();
        assert!(!heartbeat.accept_pong(1, b"wrong"));
        assert_eq!(heartbeat.deadline_ms(), 10_000);
        assert!(heartbeat.accept_pong(2, &first));
        assert!(!heartbeat.accept_pong(3, &first));
        let second = heartbeat.ping_if_due(5002).unwrap();
        assert!(!heartbeat.accept_pong(5003, &first));
        assert!(!heartbeat.accept_pong(15_002, &second));
        assert_eq!(heartbeat.deadline_ms(), 15_002);
        assert_eq!(heartbeat.ping_if_due(15_002), None);
    }

    #[test]
    fn heartbeat_late_probe_does_not_extend_stalled_io_deadline() {
        let mut heartbeat = WebSocketHeartbeat::new(0);
        let first = heartbeat.ping_if_due(0).unwrap();
        assert!(heartbeat.accept_pong(10, &first));
        assert_eq!(heartbeat.deadline_ms(), 15_010);
        assert!(heartbeat.ping_if_due(14_000).is_some());
        assert_eq!(heartbeat.deadline_ms(), 15_010);
        assert!(heartbeat.expired(15_010));
    }

    #[test]
    fn command_and_ping_are_valid_json_rpc() {
        let command =
            reply::<256>(r#"{"jsonrpc":"2.0","id":4,"method":"command","params":"status"}"#);
        assert_eq!(
            command.as_deref(),
            Some(
                r#"{"jsonrpc":"2.0","id":4,"result":{"status":"command accepted","echo":"status"}}"#
            )
        );

        let ping = reply::<128>(r#"{"jsonrpc":"2.0","id":5,"method":"ping","params":null}"#);
        assert_eq!(
            ping.as_deref(),
            Some(r#"{"jsonrpc":"2.0","id":5,"result":"pong"}"#)
        );
    }

    #[test]
    fn escaped_command_is_accepted_and_echoed_verbatim() {
        let command =
            reply::<256>(r#"{"jsonrpc":"2.0","id":6,"method":"command","params":"say \"hi\"\n"}"#);
        assert_eq!(
            command.as_deref(),
            Some(
                r#"{"jsonrpc":"2.0","id":6,"result":{"status":"command accepted","echo":"say \"hi\"\n"}}"#
            )
        );
    }

    #[test]
    fn notifications_receive_no_response() {
        assert_eq!(reply::<128>(r#"{"jsonrpc":"2.0","method":"ping"}"#), None);
    }

    #[test]
    fn malformed_and_unknown_requests_return_errors() {
        assert!(reply::<128>("{").unwrap().contains(r#""code":-32700"#));
        assert!(
            reply::<128>(r#"{"jsonrpc":"2.0","id":9,"method":"missing"}"#)
                .unwrap()
                .contains(r#""code":-32601"#)
        );
    }

    #[test]
    fn telemetry_is_a_server_notification() {
        assert_eq!(
            telemetry::<128>(15),
            r#"{"jsonrpc":"2.0","method":"telemetry","params":{"uptime_s":15}}"#
        );
    }
}
