# slint-dom example

This package demonstrates the supported Slint subset and generated Rust API.
The UI is defined in [`ui/main.slint`](ui/main.slint), while [`src/lib.rs`](src/lib.rs)
connects its callbacks and state.

From this directory, build the WebAssembly package with `wasm-pack` and serve
the files over HTTP:

```bash
wasm-pack build --target web --out-dir pkg
python -m http.server 8000
```

Then open `http://localhost:8000`.
