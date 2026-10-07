//! Lightweight, web-native rendering for a focused subset of Slint.
//!
//! [`include_ui!`] reads a `.slint` file at compile time and generates a Rust
//! component which creates ordinary DOM nodes. The parser is never linked into
//! the WebAssembly binary.

mod callback;
mod chart;
mod dom;
mod property;
#[cfg(feature = "rpc")]
pub mod rpc;

pub use callback::Callback;
pub use chart::LineChart;
pub use dom::{DomBuilder, EventBinding};
pub use domlet_macros::include_ui;
pub use property::{Property, Subscription, UpdateCycle};

#[doc(hidden)]
pub mod __private {
    pub use wasm_bindgen::JsValue;
    pub use web_sys::Element;
}
