# domlet guide

This guide explains what domlet is, how to write `.slint` files for it, what
Rust code you get from them, and what is supported today. For a five-minute
setup, start with the [README quick start](../README.md#quick-start).

## 1. What domlet is

domlet turns a user interface written in a `.slint` file into ordinary HTML
elements in the browser, driven by Rust compiled to WebAssembly.

```text
ui/main.slint ──(include_ui! at compile time)──▶ Rust code ──▶ WebAssembly ──▶ HTML in the browser
```

- The `.slint` file is read **while compiling**. The parser is not part of the
  WebAssembly file, so the result stays small.
- A `Button` becomes a real `<button>`, a `Slider` a real
  `<input type="range">`, and so on.
- Typical use: control panels and status pages for embedded devices that serve
  their own web interface.

domlet is an independent project. It is not part of Slint and contains no
Slint code; it only understands the `.slint` file format.

## 2. The `.slint` language

`.slint` is the user-interface language of the [Slint](https://slint.dev)
project: a small declarative language, separate from Rust. A file describes one
component:

```slint
import { Button } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "My device";
    in-out property <string> status: "Ready";
    callback start();

    VerticalLayout {
        Text { text: status; }
        Button { text: "Start"; clicked => { root.start(); } }
    }
}
```

Where to learn it:

- **General syntax** (components, properties, callbacks, layouts):
  the official Slint language documentation, <https://docs.slint.dev>.
- **What domlet supports**: this guide (section 4). domlet supports a
  web-focused **subset**; everything else is rejected with a compile error.
- **Editor support**: the official Slint extension for Visual Studio Code gives
  syntax highlighting and diagnostics. Its live preview renders with Slint
  itself, so it can show features that domlet does not support; the
  compiler errors from domlet are the reference.

Goal: every `.slint` file that domlet accepts should also be valid for the
official Slint compiler. The repository checks its own test and example files
with the official compiler in CI (`tests/slint-compat`). This does not cover
your files, and some input is still accepted by domlet but rejected by
Slint (see "Known differences" in section 5).

## 3. From `.slint` to Rust

In your crate (a `cdylib` built with `wasm-pack`):

```rust
use wasm_bindgen::prelude::*;

domlet::include_ui!("ui/main.slint");

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;

    let status = app.status_property();
    app.on_start(move || status.set("Running".into()));

    // Keep the component alive for the lifetime of the page.
    app.keep_alive();
    Ok(())
}
```

`include_ui!` generates a Rust type named like the exported component
(`MainWindow`). The table shows what each `.slint` declaration becomes:

| In `.slint` | Generated Rust |
|---|---|
| `export component MainWindow inherits Window` | `struct MainWindow` with `mount(&parent)`, `mount_to_body()`, `root()`, `unmount(self)`, `keep_alive(self)` |
| `in-out property <string> status` | `status() -> String`, `set_status(String)`, `status_property() -> Property<String>` |
| `property <bool> active` | `active()`, `set_active(bool)`, `active_property()` |
| `callback start();` | `on_start(impl FnMut() + 'static)` |
| `start-button := Button { ... }` (element id) | `start_button() -> &Element` |
| `title: "My device";` | Sets the browser page title |

Names with `-` become `_` in Rust (`start-button` → `start_button()`).

`Property` handles can be cloned and observed. With an additional
`in-out property <float> level: 0;` in the component above:

```rust
let level = app.level_property();
let status = app.status_property();
let _subscription = level.observe(move |value| status.set(format!("Level: {value}")));
level.set(42.0);
```

Keep the returned `Subscription` alive as long as you need updates. The
[runtime contract](runtime-contract.md) describes lifetimes and update rules in
detail.

To see the generated Rust code, install `cargo-expand` and run
`cargo expand --target wasm32-unknown-unknown` in your crate.

## 4. What is supported

### 4.1 Component level

| Feature | Notes |
|---|---|
| `export component Name inherits Window { ... }` | One component per file. `Window` becomes `<main>`. |
| `title: "...";` | Only on the component; sets the page title. |
| Properties | `property`, `in property`, `out property`, `in-out property` with types `string`, `bool`, `int` (`i32`), `float` (`f64`), optional initial value. Real Slint only exposes `in`/`out`/`in-out` to Rust, so prefer those. |
| Callbacks | `callback name();` without parameters. |
| Imports | Only `import { Button, LineEdit, CheckBox, Slider } from "std-widgets.slint";` (any subset of these four). |

### 4.2 Elements

| Element | Becomes | Own properties | Events |
|---|---|---|---|
| `Text` | `<span>` | `text` | - |
| `Button` | `<button type="button">` | `text` | `clicked` |
| `TouchArea` | `<button type="button">` | - | `clicked` |
| `LineEdit` (also `TextInput`) | `<input type="text">` | `text` (two-way), `placeholder-text` | `accepted` (Enter), `edited` (every input) |
| `CheckBox` | `<input type="checkbox">` | `checked` (two-way) | `toggled` |
| `Slider` | `<input type="range">` | `value` (two-way, `float` or `int`), `minimum`, `maximum`, `step` | - |
| `Image` | `<img>` | `source` (string; see section 5) | - |
| `Rectangle` | `<div>` | - | - |
| `VerticalLayout` | `<div>`, flex column | `padding`, `spacing` | - |
| `HorizontalLayout` | `<div>`, flex row | `padding`, `spacing` | - |

`GridLayout` is accepted but only as a basic CSS grid without rows or
columns; prefer the vertical and horizontal layouts.

Properties that work on every element:

| Property | Values |
|---|---|
| `width`, `height` | Length (see 4.4), `%` allowed |
| `min-width`, `min-height`, `max-width`, `max-height` | Length |
| `background` | Color (see 4.4) |
| `border-radius` | Length |
| `visible` | `true`, `false`, a bool property, or `!property` |
| `enabled` | Same as `visible`; only on `Button`, `TouchArea`, `LineEdit`, `CheckBox`, `Slider` |
| `accessible-label` | String |
| `accessible-role` | Unquoted Slint role, for example `button`, `image`, `text`, `slider`, `list`, `progress-indicator` |
| `accessible-live-region` | `off`, `polite`, `assertive` |

### 4.3 Values and bindings

- **Literals**: `"text"`, `true`/`false`, numbers, lengths (`12px`), colors
  (`#eef4ff`).
- **Property bindings**: `text: status;` or `text: root.status;` (the same).
  Bound values update the page when the property changes.
- **Negation**: `!property` only for `visible` and `enabled`.
- **Two-way**: `LineEdit.text`, `CheckBox.checked`, and `Slider.value` also
  write user input back into the property.
- **Event handlers** call exactly one declared callback:
  `clicked => { root.start(); }`.

### 4.4 Lengths and colors

- **Length units** (as in Slint): `px`, `phx`, `rem`, `cm`, `mm`, `in`, `pt`,
  and unitless `0`. `%` only on `width` and `height`. `phx` (physical pixels)
  is emitted as CSS `px`, which matches Slint only at a device pixel ratio
  of 1.
- **Colors**: unquoted hex (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`) or one of
  `transparent`, `black`, `white`, `red`, `green`, `blue`.
- Rejected: CSS-only units (`em`, `vh`, `vw`), quoted lengths (`"12px"`), and
  quoted colors (`"#fff"`).

## 5. What is not supported

These are rejected with a compile error that names the file, line, and column:

- callback parameters and return values;
- expressions (`a + b`, `condition ? x : y`, function calls);
- string interpolation (`"\{value}"`): build the text in Rust and bind a
  string property instead;
- repeated elements (`for`), conditional elements (`if`);
- custom components and imports other than the four standard widgets above;
- animations, states, transitions, and other Slint features not listed in
  section 4.

Known differences from real Slint (planned to be closed):

- Two-way bindings use `:` in domlet; real Slint needs `<=>` for them.
- `Image` takes a plain string; real Slint needs `@image-url("...")`.
- `background` and `border-radius` are accepted on every element; real Slint
  has them only on `Rectangle` (and `background` on `Window`).
- Component properties named like `Window` properties (for example
  `in property <bool> visible;`) are accepted; real Slint rejects them.
- Unknown string escapes (for example `"\q"`) are accepted as the plain
  character; real Slint rejects them and supports `\u{...}`.
- `root.` is accepted in front of any value, for example
  `accessible-role: root.text;` or `background: root.red;`; real Slint only
  allows it in front of property names.
- Some number spellings in lengths differ: `1e3px` is accepted by domlet
  but rejected by Slint.

## 6. Compile errors

domlet reports problems while compiling and points at the exact place:

```text
error: domlet:
       *****************************************************************
       ui/main.slint:4:9: `background` requires a color literal without quotes: write `#eef4ff`, not `"#eef4ff"`
         |
       4 |         background: "#eef4ff";
         |         ^
       *****************************************************************
```

If an error says something is "not supported", check section 4 for the
supported alternative.

## 7. Talking to the device (optional)

Enable the `rpc` feature (`domlet = { version = "...", features = ["rpc"] }`)
for a small JSON-RPC 2.0 client over WebSocket (`domlet::rpc::RpcClient`).
Request identifiers are `rpc::Id` values, either numbers or strings; plain
numbers and strings convert automatically, for example
`client.request(7, "command", &params)` or `client.request("a1", ...)`.
The [`example-server`](https://github.com/nursude355/Domlet/blob/main/example-server/README.md)
shows a desktop server and a Raspberry Pi Pico 2 + W5500 firmware that serve
the page and answer calls.

Today the calls are untyped (method names and JSON values). Typed calls in both
directions (browser calls device functions, device calls browser functions) are
being designed and are **not available yet**.

## 8. Roadmap (not available yet)

- Closing the remaining differences from real Slint (section 5).
- Typed RPC: call a Rust function on the device from the browser as
  `set_brightness(80).await`, and the reverse direction.
- More elements and language features (for example lists with `for`,
  conditional elements with `if`, more widgets) where they map well to HTML.
