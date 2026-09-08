//! Lightweight, web-native rendering for a focused subset of Slint.
//!
//! [`include_ui!`] reads a `.slint` file at compile time and generates a Rust
//! component which creates ordinary DOM nodes. The parser is never linked into
//! the WebAssembly binary.

mod callback;
mod dom;
mod property;

pub use callback::Callback;
pub use dom::{DomBuilder, EventBinding};
pub use property::Property;
pub use slint_dom_macros::include_ui;

#[doc(hidden)]
pub mod __private {
    pub use wasm_bindgen::JsValue;
    pub use web_sys::Element;
}
