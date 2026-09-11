# Example server

This is a deliberately small, local-only WebSocket/JSON-RPC server. It is an
application, not part of the crate workspace and not published.

From the repository root:

```powershell
wasm-pack build example --target web --release --out-dir pkg
cargo run --manifest-path example-server/Cargo.toml
```

It serves `example/index.html` at `http://127.0.0.1:8080` and the generated
JavaScript/WASM under `/pkg/`, opens the browser when
possible, and exposes a WebSocket endpoint at `/rpc`. The protocol accepts
`ping` and `command`; it is a working integration reference, not an
authentication or production deployment template.
