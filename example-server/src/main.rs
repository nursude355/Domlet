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
    let origin = headers.get(ORIGIN).map(|value| value.to_str().unwrap_or(""));
    if origin.is_some_and(|origin| !ALLOWED_ORIGINS.contains(&origin)) {
        return StatusCode::FORBIDDEN.into_response();
    }
    socket.on_upgrade(handle_rpc).into_response()
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
