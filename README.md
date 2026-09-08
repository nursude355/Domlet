# slint-dom

`slint-dom` compiles a focused, web-friendly subset of Slint into ordinary
browser DOM nodes. It keeps Slint's declarative UI format without linking the
Slint renderer into the WebAssembly application. The parser and Rust generator
run at compile time, so the WASM output contains only the small DOM runtime and
application code.

```text
                  ui/main.slint
                 /             \
        Slint desktop       slint-dom macro
              |                   |
          native UI         browser DOM + WASM
```

## Status

This is an experimental `0.1` library. It supports real DOM construction,
reactive component properties, two-way text/checkbox bindings, Rust callbacks,
element IDs, DOM event cleanup, literal styling, strict compile-time validation,
and WebAssembly compilation.

It is useful for prototypes and small embedded-device web interfaces whose UI
fits the documented subset. It is not yet a complete replacement for Slint's
renderer.

## Installation

```toml
[dependencies]
slint-dom = "0.1"
wasm-bindgen = "0.2"
```

The application must be compiled for `wasm32-unknown-unknown`.

## Quick start

Create `ui/main.slint`:

```slint
import { Button } from "std-widgets.slint";

export component MainWindow inherits Window {
    property <string> status: "Ready";
    property <bool> enabled: true;
    callback start();

    VerticalLayout {
        Text { text: status; }
        start-button := Button {
            text: "Start";
            enabled: enabled;
            clicked => { root.start(); }
        }
    }
}
```

Generate and mount the component:

```rust,ignore
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

slint_dom::include_ui!("ui/main.slint");

thread_local! {
    static APP: RefCell<Option<MainWindow>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;
    let status = app.status_property();
    app.on_start(move || status.set("Running".into()));
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    Ok(())
}
```

The component must remain alive while it is mounted. Dropping it unregisters
its DOM event listeners. `unmount()` additionally removes the root node.

## Generated API

For `property <bool> enabled`, the macro generates:

```rust,ignore
app.enabled();
app.set_enabled(false);
app.enabled_property();
```

For `callback start()`, it generates `app.on_start(|| { /* code */ })`. For
`start-button := Button`, it generates `app.start_button()`, which returns the
corresponding DOM element.

## Supported subset

Component declarations:

- One exported component per file
- Optional `inherits Window`
- `property <string>`, `<bool>`, `<int>`, and `<float>`
- Zero-argument callbacks
- Element IDs using `name := Widget { ... }`
- Literal strings, booleans, numbers, and direct property references

Widgets:

- Layout: `VerticalLayout`, `HorizontalLayout`, `GridLayout`
- Content: `Text`, `Image`, `Rectangle`
- Controls: `Button`, `LineEdit`, `TextInput`, `CheckBox`, `Slider`, `TouchArea`

Reactive bindings:

- `Text.text` from a string property
- `enabled` and `visible` from bool properties
- `LineEdit.text`/`TextInput.text` as two-way string bindings
- `CheckBox.checked` as a two-way bool binding

Events:

- `Button.clicked` and `TouchArea.clicked`
- `LineEdit.accepted`, `LineEdit.edited`, and their `TextInput` equivalents
- `CheckBox.toggled`

Style and attributes:

- Size: `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height`
- Layout: `padding`, `spacing`
- Appearance: `background`, `border-radius`
- Input: `placeholder-text`, `checked`, `value`, `minimum`, `maximum`
- Image: `source`

Unsupported widgets, properties, bindings, events, callback signatures, and
component members produce compile-time errors instead of being ignored.

## Current boundaries

The following remain intentionally unsupported in `0.1`:

- General Slint expressions, arithmetic, aliases, structs, models and globals
- `if`, `for`, repeaters, animations and transitions
- Callback arguments and return values
- Reactive numeric slider bindings
- Multiple components and imports of user-defined components
- Server, WebSocket and RPC functionality

WebSocket/RPC belongs in the application or a future optional transport crate;
keeping it outside the renderer allows `slint-dom` to work with different web
servers and embedded targets.

## Example and development

The [`example`](example) demonstrates reactive text, buttons, callbacks,
two-way input state, a checkbox and a slider.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p slint-dom-example --target wasm32-unknown-unknown
```

CI runs the same checks on every push and pull request.
The release profile is optimized for size, and CI rejects a raw example WASM
larger than 700 KB. The current raw artifact is about 623 KB; after normal
`wasm-bindgen` processing, the example's deployed WASM is about 36 KB without
`wasm-opt`.

## License

Licensed under the [MIT License](LICENSE-MIT).
