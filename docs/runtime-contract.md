# domlet: runtime and validation contract

For the installation and first-project guide, start with the
[README](../README.md).

`domlet` compiles a focused subset of `.slint` UI descriptions into Rust
which creates native browser DOM nodes. Compilation runs in the procedural
macro crate; parsing and code generation are not part of the deployed WASM.

The application uses `domlet::include_ui!("ui/main.slint")` and mounts the
generated component with `MainWindow::mount_to_body()`. See the repository's
`example/` application for a complete UI and callback wiring. Rust 1.81 is the
declared minimum, and applications target `wasm32-unknown-unknown`.

Supported features include direct string/bool bindings, two-way input,
checkbox and slider state, zero-argument callbacks, element IDs, accessible
labels/roles, and basic HTML controls and layouts. `!property` is supported
for boolean `enabled` and `visible` bindings. Bindings may be written with a
`root.` prefix (`root.status`, `!root.active`). `in`, `out`, and `in-out`
property declarations generate the same Rust accessors as plain `property`;
no access restriction is enforced for `out` yet. General Slint expressions,
conditional/repeated elements, callback parameters, and user-component imports
remain deliberately outside this focused subset.

Keep the generated component alive while its DOM is interactive. `unmount()`
removes its root and drops its event listeners and property subscriptions.
Dropping a component without calling `unmount()` leaves the last DOM snapshot,
but disconnects its bindings and listeners. Externally cloned properties remain
usable and no longer update that snapshot.

For applications that intentionally run for the lifetime of the browser page,
the generated `keep_alive(self)` method retains the component and all of its
bindings. This is an explicit page-lifetime allocation; it cannot later be
unmounted. Applications with a shorter lifecycle should store the component
instead and eventually call `unmount()`.

`Property::observe` returns a `Subscription`. Retain that token for as long as
updates are required. Dropping it releases the observer's captured values.
Observers receive an initial value immediately and subsequent values
synchronously in registration order.

`set_if_changed` skips observer notification when the new value equals the
current value. Generated property setters and two-way DOM bindings use this
behavior to avoid redundant update chains. `set` remains available for types
without `PartialEq` and always notifies.

`try_set` rejects an update to a property which is currently notifying its
observers, returning `UpdateCycle` without modifying that property's value.
`try_invoke` similarly rejects recursive callback invocation. The convenience
methods `set` and `invoke` panic on these programming errors. Use the fallible
methods when handling user-defined feedback. Earlier changes in a chain are
not rolled back when a later update is rejected. Generated event listeners
use `try_invoke`, and the two-way `text`, `checked`, and slider `value`
bindings use `try_set_if_changed`: an event re-dispatched synchronously while it is being
handled (for example with `element.click()` from a callback or observer) is
ignored instead of running again recursively.

With the `rpc` feature, JSON-RPC request and response identifiers use
`rpc::Id`. Numeric `u64` identifiers and owned or borrowed string identifiers
are accepted by `RpcClient::request`; incoming `Message` values preserve both
forms. Notifications continue to omit the identifier.

Only imports from `std-widgets.slint` for supported standard widgets are accepted.
Unsupported imports, duplicate members, reserved generated identifiers, invalid
CSS lengths, string interpolation (`"\{...}"`), and children of void elements
are compilation errors. Lengths
must be unquoted, non-negative finite values with a Slint unit, or unitless
zero. Supported units are `px`, `phx`, `rem`, `cm`, `mm`, `in`, and `pt`, plus
`%` on `width` and `height` (Slint converts percentages to lengths only
there). `em`, `vh`, and `vw` are rejected. Units are passed to CSS unchanged,
except `phx` (physical pixels), which is emitted as CSS `px` and therefore
equals Slint's size only at a device pixel ratio of 1. CSS and Slint use the
same factors for `cm`, `mm`, `in`, and `pt` (1in = 96px); Slint's `rem` is
relative to the window's default font size, CSS `rem` to the page's root font
size. Colors must be unquoted hex literals (`#rgb`, `#rgba`, `#rrggbb`,
`#rrggbbaa`) or one of `transparent`, `black`, `white`, `red`, `green`,
`blue`; quoted colors such as `"#fff"` are rejected.

`accepted` fires on Enter (excluding key repeats and IME composition), while
`edited` fires on input. Disabled elements do not invoke generated callbacks.
`enabled` is accepted only on interactive elements (`Button`, `TouchArea`,
`LineEdit`, `TextInput`, `CheckBox`, `Slider`).
Buttons explicitly use `type="button"` to avoid submitting enclosing forms.
Float sliders use `step="any"` unless an explicit `step` is specified. A
slider's value is applied after its bounds, whatever their source order, so
browsers do not clamp it to the default range.

Enable the optional `rpc` feature for the browser JSON-RPC 2.0 WebSocket
transport. It owns its JavaScript callbacks, disconnects on drop, and passes
decoded messages to one replaceable handler. The `example-server/` application
is a local-only working reference for `ping` and `command`; it is not part of
the published workspace or a production authentication/deployment design.
`LineChart` is a lightweight SVG telemetry helper used by the example; it
avoids a graphics runtime and has no animation loop.

Verification:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
wasm-pack test --headless --chrome
wasm-pack build example --target web --release --out-dir pkg --locked
cargo test --manifest-path example-server/Cargo.toml --locked
cargo build --manifest-path tests/slint-compat/Cargo.toml --locked
```

The last command compiles every `.slint` file in `tests/ui/` and
`example/ui/main.slint` with the official Slint compiler (`slint-build`, in a
separate workspace with its own lockfile) and fails if any of them is not
valid Slint. `tests/ui/compat.slint` is compiled by both compilers and covers
the Slint syntax domlet accepts (`in`/`out`/`in-out` properties, `root.`
bindings, length units, and color literals).

The browser suite includes a real generated component, two-way input and
checkbox updates, callback delivery, keyboard acceptance, disabled clicks, and
unmount with externally retained property handles. CI also checks the deployed
WASM size, separately from the raw compiler artifact. The example enables the
optional RPC client, so its deployed-package budget is 200 KB.

Release preparation: CI fully verifies `domlet-macros`, then packages both
workspace crates together with `--no-verify` so the root archive can be
inspected before the matching macro version exists in the registry. Workspace
and browser tests validate the local pair. Publish `domlet-macros` first,
wait until that version is available from crates.io, then run `cargo package -p
domlet --locked` for the final independent verification and publish
`domlet`. No publication is performed by the CI workflow.
