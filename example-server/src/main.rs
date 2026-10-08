//! Development server for the browser example.
//!
//! Build the browser package first with `wasm-pack build example --target web
//! --release --out-dir pkg`, then run this binary from the repository root.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{header::ORIGIN, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

/// Browser origins allowed to open the RPC socket. Browsers always send
/// `Origin` on WebSocket handshakes, so this stops other websites open in the
/// same browser from driving the local server.
const ALLOWED_ORIGINS: [&str; 2] = ["http://127.0.0.1:8080", "http://localhost:8080"];
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/rpc", get(upgrade_rpc))
        .fallback_service(ServeDir::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../example"
        )));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("port 8080 is unavailable");
    let address = "http://127.0.0.1:8080";
    println!("domlet example: {address}");
    if let Err(error) = webbrowser::open(address) {
        eprintln!("could not open browser: {error}; open {address} manually");
    }
    axum::serve(listener, app).await.expect("server failed");
}

async fn upgrade_rpc(headers: HeaderMap, socket: WebSocketUpgrade) -> Response {
    let origin = headers
        .get(ORIGIN)
        .map(|value| value.to_str().unwrap_or(""));
    if origin.is_some_and(|origin| !ALLOWED_ORIGINS.contains(&origin)) {
        return StatusCode::FORBIDDEN.into_response();
    }
    socket
        .max_message_size(domlet_example_server::RPC_MESSAGE_LIMIT)
        .max_frame_size(domlet_example_server::RPC_MESSAGE_LIMIT)
        .on_upgrade(handle_rpc)
        .into_response()
}

async fn handle_rpc(mut socket: WebSocket) {
    let mut telemetry = tokio::time::interval(std::time::Duration::from_secs(5));
    telemetry.tick().await;
    let mut uptime_seconds = 0;

    loop {
        tokio::select! {
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Some(response) = domlet_example_server::reply::<1024>(&text) {
                        if socket.send(Message::Text(response.as_str().into())).await.is_err() {
                            return;
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                Some(Ok(_)) => {}
            },
            _ = telemetry.tick() => {
                uptime_seconds += 5;
                let message = domlet_example_server::telemetry::<128>(uptime_seconds);
                if socket.send(Message::Text(message.as_str().into())).await.is_err() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::Duration,
    };

    // Exercise the real Axum upgrade without launching the example browser or
    // adding a WebSocket test dependency. The client uses masked RFC 6455 frames.
    async fn with_socket(
        origin: Option<&'static str>,
        test: impl FnOnce(TcpStream, String) + Send + 'static,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route("/rpc", get(upgrade_rpc));
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let result = tokio::task::spawn_blocking(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let origin = origin.map_or_else(String::new, |value| format!("Origin: {value}\r\n"));
            write!(
                stream,
                "GET /rpc HTTP/1.1\r\nHost: {address}\r\nUpgrade: websocket\r\n\
                 Connection: Upgrade\r\nSec-WebSocket-Version: 13\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n{origin}\r\n"
            )
            .unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                headers.push(byte[0]);
                assert!(headers.len() < 4096);
            }
            test(stream, String::from_utf8(headers).unwrap());
        })
        .await;
        server.abort();
        result.unwrap();
    }

    fn send_frame(stream: &mut TcpStream, opcode: u8, final_frame: bool, payload: &[u8]) {
        let mut frame = vec![opcode | if final_frame { 0x80 } else { 0 }];
        if payload.len() <= 125 {
            frame.push(0x80 | payload.len() as u8);
        } else {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        let mask = [1, 2, 3, 4];
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(i, byte)| byte ^ mask[i % 4]),
        );
        stream.write_all(&frame).unwrap();
    }

    fn assert_peer_closed(mut stream: TcpStream) {
        let mut received = Vec::new();
        match stream.read_to_end(&mut received) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
            Err(error) => panic!("oversized message did not close the peer: {error}"),
        }
        // Closing directly or sending a Close frame are both valid here; an
        // RPC Text response would mean an oversized request was dispatched.
        assert!(received.is_empty() || received[0] & 0x0f == 8);
    }

    #[tokio::test]
    async fn desktop_origin_allowlist_is_unchanged() {
        for origin in [None, Some(ALLOWED_ORIGINS[0]), Some(ALLOWED_ORIGINS[1])] {
            with_socket(origin, |_, headers| {
                assert!(headers.starts_with("HTTP/1.1 101 "));
            })
            .await;
        }
        for origin in ["http://evil.example", "null", "http://localhost:8080/", ""] {
            with_socket(Some(origin), |_, headers| {
                assert!(headers.starts_with("HTTP/1.1 403 "));
            })
            .await;
        }
    }

    #[tokio::test]
    async fn desktop_accepts_limit_sized_request() {
        with_socket(None, |mut stream, headers| {
            assert!(headers.starts_with("HTTP/1.1 101 "));
            let mut request = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#.to_vec();
            request.resize(domlet_example_server::RPC_MESSAGE_LIMIT, b' ');
            send_frame(&mut stream, 1, true, &request);
            let mut frame_header = [0; 2];
            stream.read_exact(&mut frame_header).unwrap();
            assert_eq!(frame_header[0], 0x81);
            assert!(frame_header[1] < 126);
            let mut response = vec![0; frame_header[1] as usize];
            stream.read_exact(&mut response).unwrap();
            assert_eq!(response, br#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#);
        })
        .await;
    }

    #[tokio::test]
    async fn desktop_rejects_oversized_frame_and_fragmented_message() {
        for fragmented in [false, true] {
            with_socket(None, move |mut stream, headers| {
                assert!(headers.starts_with("HTTP/1.1 101 "));
                let payload = [b' '; domlet_example_server::RPC_MESSAGE_LIMIT + 1];
                if fragmented {
                    send_frame(&mut stream, 1, false, &payload[..512]);
                    send_frame(&mut stream, 0, true, &payload[512..]);
                } else {
                    send_frame(&mut stream, 1, true, &payload);
                }
                assert_peer_closed(stream);
            })
            .await;
        }
    }
}
