# Changelog

All notable changes to this project are documented here. The project follows
Semantic Versioning while its public API is still below 1.0.

## [Unreleased]

### Breaking

- String interpolation (`"Count: \{count}"`) is a compile error pointing at
  the `\{`. It used to produce the literal text `{count}`; Slint would insert
  the value. Build the text in Rust and bind a string property instead.
- Lengths accept only Slint units: `px`, `phx`, `rem`, `cm`, `mm`, `in`, `pt`,
  `%` (on `width` and `height` only, as in Slint), or unitless `0`. The
  CSS-only units `em`, `vh`, and `vw` are rejected with an error naming the
  supported units. `phx` is emitted as CSS `px`.
- Colors and lengths must be unquoted literals: write `background: #eef4ff;`
  and `width: 12px;`, not `"#eef4ff"` or `"12px"`. Color literals are no
  longer accepted as string values.

### Added

- Slint's `in property`, `out property`, and `in-out property` declarations.
  They generate the same Rust API as a plain `property` (`name()`,
  `set_name()`, `name_property()`); `out` is not yet read-only from Rust.
- Values can name root properties as `root.name`, also negated
  (`visible: !root.active;`); this is the same as writing `name`.
- CI compiles every `.slint` file in `tests/ui/` and `example/ui/main.slint`
  with the official Slint compiler (`tests/slint-compat`, using
  `slint-build`), so the project's own test and example files stay valid
  Slint.
- A user guide (`docs/guide.md`, also in the crate package): what slint-dom
  is, how `.slint` declarations map to the generated Rust API, the supported
  elements, properties, and values, and the known differences from Slint.

### Fixed

- Element id errors (for example a duplicate id in `status := Text { ... }`)
  point at the id instead of at the element kind.

## [0.2.0] - 2026-09-30

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

[Unreleased]: https://github.com/nursude355/Slint_Dom/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nursude355/Slint_Dom/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nursude355/Slint_Dom/releases/tag/v0.1.0
