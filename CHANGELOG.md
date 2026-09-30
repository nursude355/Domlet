# Changelog

All notable changes to this project are documented here. The project follows
Semantic Versioning while its public API is still below 1.0.

## [0.2.0] - Unreleased

### Breaking

- `accessible-role` takes a Slint enum value instead of a quoted string:
  write `accessible-role: button;`, not `accessible-role: "button";`. Slint
  role names are mapped to their ARIA equivalents (for example `image` to
  `img`); `text` adds no role.
- `enabled` is accepted only on interactive elements (`Button`, `TouchArea`,
  `LineEdit`, `TextInput`, `CheckBox`, `Slider`). It is now a compile-time
  error on `Text`, `Rectangle`, and layouts, where it had no effect.
- `slint-dom` requires `wasm-bindgen` 0.2.129 or newer.

### Added

- `.slint` compile errors name the file, line, and column, and point at the
  offending source line. Generator errors point at the specific property,
  handler, or element and are framed so they stand out in build output.
- `accessible-live-region` (`off`, `polite`, `assertive`).
- `LineChart::set_points_in_range` plots values on a fixed scale, so the line
  reflects absolute values instead of being normalized to its own range.
- `RpcClient::is_open` reports whether the WebSocket can send; creating a
  client does not fail for an unreachable server.

### Fixed

- A slider's bound `value` is applied after `minimum`, `maximum`, and `step`,
  so browsers no longer clamp it to the default range when `value` is written
  first.
- An event that is re-dispatched synchronously while it is being handled (for
  example with `element.click()` from a callback or property observer) is
  ignored instead of aborting the WebAssembly module. This covers generated
  callbacks and the two-way `text`, `checked`, and slider `value` bindings.
- A property initial value with the wrong type is reported at the property
  instead of the end of the file, and out-of-range float literals such as
  `1e999` are rejected with a located error instead of crashing the macro.
- The default stylesheet's `[hidden]` rule applies only inside generated
  components instead of the whole page.
- An RPC message handler can replace itself from inside the running handler.
- `LineChart` keeps a constant stroke width when its host is resized.

## [0.1.0] - 2026-09-10

- Initial crates.io release.

[0.2.0]: https://github.com/nursude355/Slint_Dom/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nursude355/Slint_Dom/releases/tag/v0.1.0
