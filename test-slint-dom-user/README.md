This is the root README quick start as a runnable consumer. It uses the local
crate so CI tests changes before publication. For a registry-only reproduction,
replace the path dependency with `slint-dom = "0.1"`.

From the repository root:

```sh
wasm-pack build test-slint-dom-user --target web --release --out-dir pkg
python -m http.server 8000 --directory test-slint-dom-user
```

Open http://localhost:8000. The page should show Ready and a Start button.
Click Start: Ready must change to Running. The HTML file on disk stays unchanged;
the generated WASM adds elements to the browser DOM.
