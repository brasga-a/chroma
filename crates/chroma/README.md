# chroma

Rust library for deterministic RGB composition. It has no interface,
manufacturer-specific code, device transport, background thread, or clock. A
client supplies a layout and timestamp and consumes the resulting frame.

```sh
cargo test -p chroma
cargo run -p chroma --example direct
cargo run -p chroma --example json_lines
```

`direct` demonstrates the Rust API and layered effects. `json_lines` provides a
version 1 request/response adapter over stdin/stdout that can be wrapped by a
CLI or other client. It starts with an empty scene and a synthetic 6×22 geometry
template; generated IDs are not a physical device mapping.

See [`../../docs/engine.md`](../../docs/engine.md) for the API and protocol and
[`../../docs/chroma.md`](../../docs/chroma.md) for the architecture boundaries.
The engine makes no device compatibility claim. Device adapters and user
interfaces are optional external consumers.
