//! Small web UIs for embedded devices: a focused subset of the `.slint` UI
//! language, compiled to native browser DOM with Rust/WASM.
//!
//! [`include_ui!`] reads a `.slint` file at compile time and generates a Rust
//! component which creates ordinary DOM nodes. The parser is never linked into
//! the WebAssembly binary.
//!
//! The crate is intended for small Rust/WASM control panels, telemetry
//! displays, and embedded-device web UIs that can run complex code (such as
//! generating statistics or charts, coded in Rust) in the browser, and want
//! to use the `.slint` language for the UI layout and styling without the full
//! official Slint compilation. It generates DOM widgets instead of rendering
//! widgets in WASM, which drastically reduces the size of the WASM binary.
//!
//! A [WASM example] shows how to use the crate to create a simple line chart
//! and some widgets with a `.slint` UI.
//!
//! A [server example] provides a web server application that the WASM example
//! can connect to. It shows sending unsolicited messages in both directions
//! over WebSockets using an RPC protocol. The server example can be compiled
//! for a desktop development computer as well as for a Raspberry Pi Pico 2
//! with the W5500 Ethernet module, and run on the device.
//!
//! The examples are in the GitHub repository, not in the published crate. The
//! [README](https://github.com/nursude355/Domlet#readme) has a quick start, and
//! the [guide](https://github.com/nursude355/Domlet/blob/main/docs/guide.md)
//! lists the supported `.slint` subset.
//!
//! [WASM example]: https://github.com/nursude355/Domlet/tree/main/example
//! [server example]: https://github.com/nursude355/Domlet/tree/main/example-server
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
