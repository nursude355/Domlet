# Example servers

This package provides the same web UI and WebSocket/JSON-RPC behavior on two
targets:

- `desktop` (default): Tokio + Axum development server.
- `embedded`: allocation-free Embassy server for Raspberry Pi Pico 2
  (RP2350A) with an SPI W5500 Ethernet module.

It is an application, not part of the publishable crate workspace.

## Desktop (Tokio)

From the repository root:

```powershell
wasm-pack build example --target web --release --out-dir pkg
cargo run --manifest-path example-server/Cargo.toml
```

It serves `example/index.html` at `http://127.0.0.1:8080` and the generated
JavaScript/WASM under `/pkg/`, opens the browser when
possible, and exposes a WebSocket endpoint at `/rpc`. The protocol accepts
`ping` and `command`. The server also sends a `telemetry` notification every
five seconds, demonstrating unsolicited server-to-browser messages.

## Raspberry Pi Pico 2 + W5500 (Embassy)

The embedded binary uses `#![no_std]`, fixed-capacity buffers, `embassy-net`,
and picoserve. HTML, JavaScript, and WebAssembly are compiled into flash. No
heap allocator is installed or used.

The wiring follows Embassy's W5500 reference:

| Pico 2 | W5500 |
| --- | --- |
| GP16 | MISO |
| GP17 | CS |
| GP18 | SCK |
| GP19 | MOSI |
| GP20 | RESET |
| GP21 | INT |
| 3V3 | VCC |
| GND | GND |

Use a 3.3 V-compatible W5500 module. The example uses SPI0 at 50 MHz, DHCP,
TCP port 80, and MAC address `02:00:00:00:00:02`. Change the MAC if multiple
devices will share a network.

Install the target and build the browser assets first:

```powershell
rustup target add thumbv8m.main-none-eabihf
wasm-pack build example --target web --release --out-dir pkg --locked
```

Then build the firmware with Rust 1.93 or newer:

```powershell
cargo build --manifest-path example-server/Cargo.toml `
  --no-default-features --features embedded `
  --bin slint-dom-pico2-w5500 `
  --target thumbv8m.main-none-eabihf --release --locked
```

The ELF firmware is written to:

```text
example-server/target/thumbv8m.main-none-eabihf/release/slint-dom-pico2-w5500
```

Flash it with an RP2350-compatible probe/runner. RTT output prints the DHCP
address as `open http://a.b.c.d`. Opening that address loads the same WASM UI
from the Pico 2 and connects `/rpc` over WebSocket.

This is a LAN integration reference, not an authenticated or TLS-enabled
production server. The embedded example serves one connection at a time; after
the browser assets are delivered, the WebSocket intentionally remains open.
