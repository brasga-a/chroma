# Chroma

RGB lighting composition engine in Rust. The engine is independent of hardware
manufacturer, interface, and transport. It accepts configurable layouts, scenes,
timestamps, and optional key-press events, then returns RGB frames. A CLI,
desktop app, web service, or device adapter can consume that API separately.

## Engine

The `chroma` crate provides:

- Ordered scenes with layers, key masks, opacity, and `over`, `add`, and
  `multiply` blending.
- Static, breathing, spectrum, wave, reactive, and ripple effects.
- Deterministic rendering at caller-supplied timestamps and bounded optional
  key-event history.
- Configurable 2D layouts with stable key/LED IDs, sparse cells, and RGB frames.
- Typed Rust commands and a version 1 JSON request/response protocol.

It does not read a clock, start threads, or access hardware. Clients decide
whether to provide key events, when to render, and where to send each frame.

## Run

The default workspace commands build and test the engine:

```sh
cargo test --locked
cargo run --locked -p chroma --example direct
cargo run --quiet --locked -p chroma --example json_lines
```

`json_lines` accepts one JSON request per line on stdin and writes one response
per line. For example:

```json
{"version":1,"command":{"type":"render_at","time_ms":1000}}
```

The example starts with a synthetic 6×22 geometry template. Its generated IDs
do not represent a verified physical key/LED map for any device.

## Documentation

- [Project architecture and boundaries](docs/chroma.md)
- [API, protocol, effects, and validation](docs/engine.md)
- [Crate and examples](crates/chroma/README.md)

The engine has no manufacturer-specific requirements. A future consumer could
be a CLI, web service, or desktop interface; an Omarchy/Quickshell interface
could be one such client. Device adapters are also separate consumers: for
example, OpenRazer may be an option for some Razer devices. Other brands such
as HyperX would require their own suitable adapter. These are integration
possibilities, not implementations or compatibility claims.

## Repository layout

- `crates/chroma/`: the independent Rust engine and its examples.
- `adapters/`: hardware communicators, separate from the engine.
- `apps/`: CLI, desktop, and web clients, separate from the engine.
- `docs/`: engine architecture and API documentation.
