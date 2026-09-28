# slint-dom: runtime and validation contract

For the installation and first-project guide, start with the
[README](../README.md).

`slint-dom` compiles a focused subset of `.slint` UI descriptions into Rust
which creates native browser DOM nodes. Compilation runs in the procedural
macro crate; parsing and code generation are not part of the deployed WASM.

The application uses `slint_dom::include_ui!("ui/main.slint")` and mounts the
generated component with `MainWindow::mount_to_body()`. See the repository's
`example/` application for a complete UI and callback wiring. Rust 1.81 is the
declared minimum, and applications target `wasm32-unknown-unknown`.

Supported features include direct string/bool bindings, two-way input,
checkbox and slider state, zero-argument callbacks, element IDs, accessible
labels/roles, and basic HTML controls and layouts. `!property` is supported
for boolean `enabled` and `visible` bindings. General Slint expressions,
conditional/repeated elements, callback parameters, and user-component imports
remain deliberately outside this focused subset.

Keep the generated component alive while its DOM is interactive. `unmount()`
removes its root and drops its event listeners and property subscriptions.
Dropping a component without calling `unmount()` leaves the last DOM snapshot,
but disconnects its bindings and listeners. Externally cloned properties remain
usable and no longer update that snapshot.

`Property::observe` returns a `Subscription`. Retain that token for as long as
updates are required. Dropping it releases the observer's captured values.
Observers receive an initial value immediately and subsequent values
synchronously in registration order.

`try_set` rejects an update to a property which is currently notifying its
observers, returning `UpdateCycle` without modifying that property's value.
`try_invoke` similarly rejects recursive callback invocation. The convenience
methods `set` and `invoke` panic on these programming errors. Use the fallible
methods when handling user-defined feedback. Earlier changes in a chain are
not rolled back when a later update is rejected. Generated event listeners
use `try_invoke`: a handler which synchronously re-dispatches its own event
(for example with `element.click()`) does not run again recursively.

Only imports from `std-widgets.slint` for supported standard widgets are accepted.
Unsupported imports, duplicate members, reserved generated identifiers, invalid
CSS lengths, and children of void elements are compilation errors. CSS lengths
must be non-negative finite values with a supported unit, or unitless zero.

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
```

The browser suite includes a real generated component, two-way input and
checkbox updates, callback delivery, keyboard acceptance, disabled clicks, and
unmount with externally retained property handles. CI also checks the deployed
WASM size, separately from the raw compiler artifact. The example enables the
optional RPC client, so its deployed-package budget is 200 KB.

Release preparation: verify the macro package with `cargo package -p
slint-dom-macros`, then publish that dependency before packaging and publishing
`slint-dom`. Packaging the root crate against crates.io cannot succeed until
the matching macro version is available there. No publication is performed by
the CI workflow.
