# Chroma architecture

Chroma is an RGB lighting composition engine. Its core has no dependency on a
hardware manufacturer, device transport, desktop shell, GUI toolkit, CLI, or
web framework. It receives data and time from a caller, composes a frame, and
returns that frame through Rust types or the version 1 JSON protocol.

Chroma defines its own rendering and composition rules. Hardware compatibility
is the responsibility of each external adapter.

## Motor boundary

The implemented crate is [`chroma`](../crates/chroma/README.md). Its core
responsibilities are:

- Configurable two-dimensional layouts of keys and LEDs with stable IDs.
- Ordered scenes, layers, masks, opacity, and RGB/RGBA blending.
- Static, breathing, spectrum, wave, reactive, and ripple effects.
- Deterministic rendering at the absolute timestamp supplied by the caller.
- Optional key-press events for reactive and ripple effects, kept in bounded
  history.
- Typed Rust commands and a version 1 JSON request/response protocol.

The engine does not read a clock, schedule a render loop, start a service, open a
device, discover hardware, or send frames. Key events are optional: static and
spatial effects work without an input source. Layouts can contain unmapped
matrix cells and LEDs with no key association; spatial effects can still use
their coordinates. A nonempty per-key mask and a key-specific Reactive effect
only select LEDs associated with keys.

The coordinate convention, composition formulas, effect timelines, validation
limits, event retention, and JSON examples are documented in
[`engine.md`](engine.md).

## Optional consumers

Any application that can construct a layout and consume an RGB frame can call
the engine. Possible consumers include a direct Rust application, CLI, desktop
editor, or web service. Those consumers own their own UI, scheduling, storage,
and process lifecycle.

A separate device adapter may translate stable LED IDs and RGB cells to a
manufacturer or daemon API. OpenRazer could be one option for supported Razer
devices; this repository does not currently implement that adapter. Other
devices, including possible HyperX integrations, would need an appropriate
consumer and adapter of their own. These examples do not establish device
support or promise an implementation.

An Omarchy/Quickshell interface could also be built as an optional client. The
engine has no Omarchy or Qt dependency, and no shell plugin or editor is part
of the current motor.

## Layout status and limits

`Layout::geometric_6x22_template()` supplies a synthetic 6×22 grid for examples.
Its generated IDs are placeholders and are not a verified physical mapping for
any keyboard. A client must provide a suitable layout for its own consumer.
This project has not validated a physical device, and the engine does not claim
parity with any branded lighting suite.

The motor currently caps each dimension at 64, matrix area at 4,096 cells, LEDs
at 1,024, layers at 32, and retained events at 512 (256 by default). The limits
bound validation and rendering work. Invalid coordinates, duplicate IDs,
malformed parameters, and incompatible configuration replacements return
typed errors without partial state changes.

## Repository layout

The engine lives in `crates/chroma/`. Hardware communicators belong in
`adapters/`, and user interfaces and other clients belong in `apps/`. Neither
category is a dependency of the engine. Each future Rust package can be added
to the workspace when it is implemented.
