#![no_std]

//! Allocation-free JSON-RPC helpers shared by the desktop and embedded servers.

use core::fmt::Write as _;
use heapless::String;
use serde::{Deserialize, Serialize};
use serde_json_core::str::EscapedStr;

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
        let command = reply::<256>(
            r#"{"jsonrpc":"2.0","id":6,"method":"command","params":"say \"hi\"\n"}"#,
        );
        assert_eq!(
            command.as_deref(),
            Some(
                r#"{"jsonrpc":"2.0","id":6,"result":{"status":"command accepted","echo":"say \"hi\"\n"}}"#
            )
        );
    }

    #[test]
    fn notifications_receive_no_response() {
        assert_eq!(
            reply::<128>(r#"{"jsonrpc":"2.0","method":"ping"}"#),
            None
        );
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
