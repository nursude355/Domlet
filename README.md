# slint-dom

`slint-dom` turns a small, web-focused subset of a `.slint` UI into ordinary
browser DOM elements. The `.slint` parser is used only while compiling; it is
not included in the deployed WebAssembly file.

It is intended for small Rust/WASM control panels, telemetry displays, and
embedded-device web UIs where a full browser renderer is unnecessary.

## Quick start

Follow these steps to see a **Ready** label and a **Start** button at
<http://localhost:8000>. You need Rust and Python 3 installed.
No separate Slint installation or RPC server is needed for this example.

On Windows with the MSVC Rust toolchain, also install Visual Studio Build Tools
with Desktop development with C++ (MSVC x64/x86 tools and a Windows SDK).
If Cargo reports `link.exe not found`, fix that toolchain installation first;
no usable browser package has been built yet.

### 1. Create your project

Run these commands in a terminal:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cargo new --lib my-web-ui
cd my-web-ui
mkdir ui
```

Keep this terminal in `my-web-ui` for all remaining commands.

### 2. Configure Cargo

Replace the entire `Cargo.toml` with:

```toml
[package]
name = "my-web-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
slint-dom = "0.1"
wasm-bindgen = "0.2"
```

### 3. Define the UI

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

### 4. Connect the UI

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

### 5. Create the web page

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
      const { default: init } = await import("./pkg/app.js");
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

### 6. Build the UI

```bash
wasm-pack build --target web --release --out-dir pkg --out-name app
```

Wait for this command to finish successfully before continuing. It creates
`pkg/app.js` and `pkg/app_bg.wasm`. The fixed output name matches the HTML above,
even if you later rename your Rust package.

### 7. Open your website

In the same `my-web-ui` directory, run:

```bash
python -m http.server 8000
```

Use `python3` if that is your Python command. Leave the terminal running and
open <http://localhost:8000> in your browser. You should see **Ready** and
**Start**. Click Start: the label changes to **Running**.

To change your UI, edit `ui/main.slint`, run step 6 again in a second terminal
in `my-web-ui`, and refresh the page.

### If the UI does not appear

- **Loading UI...** or **UI failed to load**: ensure step 6 succeeded. Open
  <http://localhost:8000/pkg/app.js>; it must show JavaScript, not a 404 error.
- **A file listing or an empty page**: serve the directory containing your new
  `index.html` and `pkg/`. Do not start the server inside `pkg/`. Hard-refresh
  the page to replace an older cached loader.
- **A build error**: resolve it before starting the server. `cargo build` alone
  does not produce the browser package; use the command in step 6.

The HTML loads the UI at runtime, so its source file stays small. If the page
still fails, include the browser Console error and the wasm-pack output when
reporting the problem.

A runnable copy is in [`test-slint-dom-user/`](test-slint-dom-user/README.md).

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
cargo test -p slint-dom-macros parser::tests
wasm-pack build --target web --release --out-dir pkg
```

For runtime guarantees and release details, read
[the runtime contract](docs/runtime-contract.md).
