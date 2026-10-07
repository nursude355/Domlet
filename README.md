# Domlet

*We cooked. 🍳 Your `.slint` UI, served as plain DOM straight from the
compiler.*

> `domlet` is an independent project. It is not affiliated with or
> endorsed by Slint / SixtyFPS GmbH and contains no Slint code.

`domlet` turns a small, web-focused subset of a `.slint` UI into ordinary
browser DOM elements. domlet has its own, independently written `.slint`
parser (not Slint's). It runs only while compiling and is not included in the
deployed WebAssembly file.

It is intended for small Rust/WASM control panels, telemetry displays, and
embedded-device web UIs where a full browser renderer is unnecessary.

## What domlet can do

- **Write the UI in `.slint`**, compile it to real HTML elements: `Text`,
  `Button`, `LineEdit`, `CheckBox`, `Slider`, `Image`, `Rectangle`,
  `TouchArea`, vertical and horizontal layouts.
- **Use it from Rust**: `include_ui!` generates a typed component with getters,
  setters, observable properties, and callback handlers.
- **Two-way bindings** for text inputs, checkboxes, and sliders; `visible` and
  `enabled` bindings, including `!property`.
- **Sizes, colors, and accessibility**: Slint length units, hex colors,
  screen-reader labels, roles, and live regions.
- **Located compile errors**: unsupported input is rejected with file, line,
  and column.
- **Small output**: the parser runs only at compile time; the example UI ships
  as roughly 150 KB of WebAssembly.
- **Optional WebSocket/JSON-RPC client** for talking to a device, a lightweight
  SVG line chart, and example servers for the desktop (Axum) and a Raspberry
  Pi Pico 2 + W5500 (`no_std`, no heap).
- **Aims at Slint compatibility**: the repository's test and example `.slint`
  files are also compiled with the official Slint compiler in CI; known
  differences are listed in the [guide](docs/guide.md#5-what-is-not-supported).

How to write `.slint` files for domlet, what each declaration becomes in
Rust, and the full list of supported features: see the
[guide](docs/guide.md).

## Planned

Not available yet:

- Closing the remaining differences from Slint (`<=>` two-way bindings,
  `@image-url`, element-specific properties).
- Typed RPC: call a Rust function on the device from the browser
  (`set_brightness(80).await`), and let the device call browser functions.
- More elements and language features (for example `for` lists and `if`
  conditions) where they map well to HTML.

## Quick start

Follow these steps to see a **Ready** label and a **Start** button at test server
<http://localhost:8000>. You need Rust and Python 3 installed.
No separate Slint installation or RPC server is needed for this example.
For `.slint` syntax highlighting, diagnostics, and previews, install the
[official Slint extension for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=Slint.slint).

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
domlet = "0.2"
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
        Text {
            text: status;
            accessible-role: text;
            accessible-live-region: polite;
        }
        Button { text: "Start"; clicked => { root.start(); } }
    }
}
```

### 4. Connect the UI

Replace `src/lib.rs`:

```rust
use wasm_bindgen::prelude::*;

domlet::include_ui!("ui/main.slint");

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;
    let status = app.status_property();
    app.on_start(move || status.set("Running".into()));

    // Keep the generated component alive for the lifetime of the page.
    app.keep_alive();
    Ok(())
}
```

`include_ui!` generates the Rust type `MainWindow` from the `.slint` file.
The UI becomes visible only when that generated component is mounted, here via
`MainWindow::mount_to_body()`. Calling `keep_alive()` intentionally retains the
component and its event bindings for a page-lifetime application. Applications
that manage their own lifecycle should store the component and call `unmount()`
when it is no longer needed.

### 5. Create the web page

Create `my-web-ui/index.html`, next to `my-web-ui/Cargo.toml` (the root of the
new application, not the root of another repository):

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

A runnable copy is in [`test-domlet-user/`](test-domlet-user/README.md).

## Supported `.slint` subset

`Text`, `Button`, `LineEdit`, `CheckBox`, `Slider`, `Image`, `Rectangle`,
`TouchArea`, `VerticalLayout`, and `HorizontalLayout` are supported. String,
boolean, integer, and float properties work; text, checkbox, and slider values
can be two-way bound. Use zero-argument callbacks, element IDs, basic sizing
and colors, `accessible-label`, `accessible-role`, `accessible-live-region`,
and `!property` with `enabled` or `visible`. Slint enum values are unquoted:
write `accessible-role: image;`, not `accessible-role: "image";`.

Properties can be declared as `property`, `in property`, `out property`, or
`in-out property`; domlet generates the same Rust API for all of them
(`status()`, `set_status()`, `status_property()`). The official Slint
compiler only exposes `in`, `out`, and `in-out` properties to Rust, so declare
every property your Rust code uses with one of these if the same `.slint`
file should also work with Slint. A value may name a property with or without
`root.`: `text: root.status;` and `text: status;` are the same.

Lengths use Slint units: `px`, `phx`, `rem`, `cm`, `mm`, `in`, `pt`, `%` (only
on `width` and `height`, as in Slint), or a unitless `0`. CSS-only units such
as `em`, `vh`, and `vw` are rejected. `phx` (physical pixels) is emitted as CSS
`px`, so it matches Slint only at a device pixel ratio of 1. Colors are
unquoted: `background: #eef4ff;` or a named color (`transparent`, `black`,
`white`, `red`, `green`, `blue`), not `"#eef4ff"`.

The repository's CI compiles its test and example `.slint` files with the
official Slint compiler as well (`tests/slint-compat`).

This is not a replacement for every Slint feature: callback parameters, general
expressions, string interpolation (`"\{value}"`), repeaters, conditionals,
custom components, and arbitrary imports are deliberately rejected with
compile-time errors.

## Optional WebSocket/RPC

Enable `domlet = { version = "0.2", features = ["rpc"] }` to use the small
browser JSON-RPC 2.0 transport. Requests accept numeric or string IDs through
`rpc::Id`, and incoming messages preserve either form. The complete local
server and command-line/telemetry example is in
[`example-server/`](example-server/README.md). It can run either as a
Tokio/Axum desktop server or as a `no_std`, no-heap Embassy server on a
Raspberry Pi Pico 2 with W5500 Ethernet.

## Before publishing an application

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -p domlet-macros parser::tests
wasm-pack build --target web --release --out-dir pkg
```

For runtime guarantees and release details, read
[the runtime contract](docs/runtime-contract.md).
