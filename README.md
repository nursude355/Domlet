# slint-dom

`slint-dom` turns a small, web-focused subset of a `.slint` UI into ordinary
browser DOM elements. The `.slint` parser is used only while compiling; it is
not included in the deployed WebAssembly file.

This crate does not generate or rewrite an HTML file. You provide a small HTML
loader; the generated JavaScript loads the WASM, and Rust creates the UI in the
browser DOM at runtime. Use the browser's Elements inspector to see those nodes;
View Page Source and the HTML file on disk still show only the loader.

It is intended for small Rust/WASM control panels, telemetry displays, and
embedded-device web UIs where a full browser renderer is unnecessary.

## Quick start

Prerequisites: Rust **1.81+**, the `wasm32-unknown-unknown` target, and
[`wasm-pack`](https://wasm-bindgen.github.io/wasm-pack/).

On Windows with the MSVC Rust toolchain, also install Visual Studio Build Tools
with Desktop development with C++ (MSVC x64/x86 tools and a Windows SDK).
If Cargo reports `link.exe not found`, fix that toolchain installation first;
no usable browser package has been built yet.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cargo new --lib my-web-ui
cd my-web-ui
```

### 1. Add dependencies

Put this in `Cargo.toml`. The `cdylib` type makes the crate buildable for the
browser.

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
slint-dom = "0.1"
wasm-bindgen = "0.2"
```

### 2. Define the UI

Create `ui/main.slint`:

```slint
import { Button } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "My web UI";
    property <string> status: "Ready";
    callback start();

    VerticalLayout {
        Text { text: status; accessible-role: "status"; }
        Button { text: "Start"; clicked => { root.start(); } }
    }
}
```

### 3. Mount it from Rust

Replace `src/lib.rs`:

```rust
use wasm_bindgen::prelude::*;

slint_dom::include_ui!("ui/main.slint");

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;
    let status = app.status_property();
    app.on_start(move || status.set("Running".into()));

    // Keep the generated component alive for the lifetime of the page.
    std::mem::forget(app);
    Ok(())
}
```

For applications with more state, store the component in a `thread_local!`
slot instead of using `mem::forget`; see [`example/src/lib.rs`](example/src/lib.rs).

### 4. Add the minimal HTML loader

Create `index.html` in the project root:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>My web UI</title>
</head>
<body>
  <p id="loading" role="status">Loading UI...</p>
  <noscript>This UI requires JavaScript and WebAssembly.</noscript>
  <script type="module">
    const loading = document.getElementById("loading");
    try {
      const { default: init } = await import("./pkg/my_web_ui.js");
      await init();
      loading.remove();
    } catch (error) {
      loading.setAttribute("role", "alert");
      loading.textContent = "UI failed to load. Build with wasm-pack and serve this directory over HTTP. " + error;
      console.error(error);
    }
  </script>
</body>
</html>
```

The JavaScript file name follows the Rust package name with hyphens converted
to underscores.

### 5. Build and open it

```bash
wasm-pack build --target web --release --out-dir pkg
python -m http.server 8000
```

Open <http://localhost:8000>. Use an HTTP server; opening `index.html` directly
from the filesystem prevents normal WASM module loading in many browsers.

The page should show **Ready** and a **Start** button. Clicking Start changes
the text to **Running**. `cargo build` or `cargo test` alone does not produce
the JavaScript loader package. After a successful wasm-pack build, `pkg/`
contains `my_web_ui.js` and `my_web_ui_bg.wasm`. Serve the project root
containing `index.html`, not just `pkg/`.

If the page reports a loading error, check the browser Console and Network tabs:
both package files must load successfully. Check that the import name matches
your package (or your custom `[lib] name`), rebuild after source changes, and
reload the page. A runnable copy of this flow is in
[`test-slint-dom-user/`](test-slint-dom-user/README.md).

## Supported `.slint` subset

`Text`, `Button`, `LineEdit`, `CheckBox`, `Slider`, `Image`, `Rectangle`,
`TouchArea`, `VerticalLayout`, and `HorizontalLayout` are supported. String,
boolean, integer, and float properties work; text, checkbox, and slider values
can be two-way bound. Use zero-argument callbacks, element IDs, basic sizing
and colors, `accessible-label`, `accessible-role`, and `!property` with
`enabled` or `visible`.

This is not a replacement for every Slint feature: callback parameters, general
expressions, repeaters, conditionals, custom components, and arbitrary imports
are deliberately rejected with compile-time errors.

## Optional WebSocket/RPC

Enable `slint-dom = { version = "0.1", features = ["rpc"] }` to use the small
browser JSON-RPC 2.0 transport. The complete local server and command-line/
telemetry example is in [`example-server/`](example-server/README.md).

## Before publishing an application

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
wasm-pack build --target web --release --out-dir pkg
```

For runtime guarantees and release details, read
[the runtime contract](docs/runtime-contract.md).
