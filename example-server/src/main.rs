//! Development server for the browser example.
//!
//! Build the browser package first with `wasm-pack build example --target web
//! --release --out-dir pkg`, then run this binary from the repository root.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/rpc", get(upgrade_rpc))
        .fallback_service(ServeDir::new("example/pkg"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("port 8080 is unavailable");
    let address = "http://127.0.0.1:8080";
    println!("slint-dom example: {address}");
    if let Err(error) = webbrowser::open(address) {
        eprintln!("could not open browser: {error}; open {address} manually");
    }
    axum::serve(listener, app).await.expect("server failed");
}

async fn upgrade_rpc(socket: WebSocketUpgrade) -> impl IntoResponse {
    socket.on_upgrade(handle_rpc)
}

async fn handle_rpc(mut socket: WebSocket) {
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        let response = match serde_json::from_str::<Value>(&text) {
            Ok(request) => reply(request),
            Err(error) => json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": error.to_string() }
            }),
        };
        if socket
            .send(Message::Text(response.to_string()))
            .await
            .is_err()
        {
            return;
        }
    }
}

fn reply(request: Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    match request.get("method").and_then(Value::as_str) {
        Some("command") => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "status": "command accepted", "echo": request.get("params") }
        }),
        Some("ping") => json!({ "jsonrpc": "2.0", "id": id, "result": "pong" }),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": "method not found" }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_a_valid_rpc_method() {
        let answer =
            reply(json!({ "jsonrpc": "2.0", "id": 4, "method": "command", "params": "status" }));
        assert_eq!(answer["id"], 4);
        assert_eq!(answer["result"]["status"], "command accepted");
    }
}
