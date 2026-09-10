//! A deliberately small JSON-RPC 2.0 WebSocket transport for browser clients.
//!
//! The module owns all JavaScript closures and closes the socket on drop. It
//! transports JSON-RPC messages; application method names and payload schemas
//! intentionally remain the application's responsibility.

use std::{cell::RefCell, rc::Rc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{MessageEvent, WebSocket};

type MessageHandler = Rc<RefCell<Option<Box<dyn FnMut(Message)>>>>;

#[derive(Debug, Serialize)]
struct Outbound<'a, T: Serialize> {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    method: &'a str,
    params: &'a T,
}

/// An incoming JSON-RPC notification or response.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Message {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<Value>,
}

/// A browser WebSocket that sends and receives JSON-RPC 2.0 messages.
pub struct RpcClient {
    socket: WebSocket,
    handler: MessageHandler,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl RpcClient {
    /// Opens a WebSocket. Use `ws://` or `wss://` according to the hosting page.
    pub fn connect(url: &str) -> Result<Self, JsValue> {
        let socket = WebSocket::new(url)?;
        let handler: MessageHandler = Rc::default();
        let message_handler = handler.clone();
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else {
                return;
            };
            let Ok(message) = serde_json::from_str::<Message>(&text) else {
                return;
            };
            if let Some(handler) = message_handler.borrow_mut().as_mut() {
                handler(message);
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Ok(Self {
            socket,
            handler,
            _on_message: on_message,
        })
    }

    /// Sets the sole incoming-message handler. Replacing it is safe.
    pub fn on_message(&self, handler: impl FnMut(Message) + 'static) {
        *self.handler.borrow_mut() = Some(Box::new(handler));
    }

    /// Sends a JSON-RPC notification (without an id).
    pub fn notify<T: Serialize>(&self, method: &str, params: &T) -> Result<(), JsValue> {
        self.send(None, method, params)
    }

    /// Sends a JSON-RPC request. Responses are delivered to [`Self::on_message`].
    pub fn request<T: Serialize>(&self, id: u64, method: &str, params: &T) -> Result<(), JsValue> {
        self.send(Some(id), method, params)
    }

    fn send<T: Serialize>(&self, id: Option<u64>, method: &str, params: &T) -> Result<(), JsValue> {
        let text = serde_json::to_string(&Outbound {
            jsonrpc: "2.0",
            id,
            method,
            params,
        })
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.socket.send_with_str(&text)
    }
}

impl Drop for RpcClient {
    fn drop(&mut self) {
        self.socket.set_onmessage(None);
        let _ = self.socket.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbound_message_is_json_rpc() {
        let encoded = serde_json::to_string(&Outbound {
            jsonrpc: "2.0",
            id: Some(7),
            method: "command",
            params: &"status",
        })
        .unwrap();
        assert_eq!(
            encoded,
            r#"{"jsonrpc":"2.0","id":7,"method":"command","params":"status"}"#
        );
    }
}
