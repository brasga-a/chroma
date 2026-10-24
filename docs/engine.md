# Chroma engine

`chroma` is a Rust library for composing RGB frames. It can be called directly
by Rust code or through the example's versioned JSON-lines adapter. It does not
own a clock, schedule a render loop, open a device, or depend on a manufacturer,
transport, GUI, asynchronous-runtime, GPU, Qt, or Linux API. The caller chooses
when to render and how to consume a returned frame.

The engine implements its own effect and composition semantics. Physical
hardware validation belongs to each device adapter.

## Layout and frame coordinates

A `Layout` has a stable layout ID, width, height, key positions, and LED
positions. Coordinates are integer matrix cells: `(0, 0)` is the upper-left,
`x` increases to the right, and `y` increases downward. Both key and LED
coordinates must be unique within their own lists and within the declared
dimensions. Each LED has its own stable ID and may refer to a key ID. An LED
without a key ID can receive spatial effects but cannot be selected by a
per-key mask or receive a key-specific Reactive flash.

The output `Frame` is a row-major matrix with `width * height` entries. A cell
contains its LED ID, optional key ID, and final RGB bytes. `null` means there is
no LED at that coordinate; holes stay holes. LED IDs and matrix coordinates
give a future consumer enough identity to map cells to its own output.

The engine limits each dimension to 64, the matrix to 4,096 cells, and the
layout to 1,024 LEDs. IDs must be nonempty, trimmed, free of control characters,
and no longer than 128 UTF-8 bytes. Coordinates outside the matrix, duplicate
IDs, duplicate coordinates, and LED references to missing keys are rejected.

`Layout::geometric_6x22_template()` creates a full 6×22 synthetic grid with
generated `template-key-*` and `template-led-*` IDs. It is useful for examples
and geometry experiments only. It is not a recovered or verified physical
mapping for any keyboard; a consumer must provide its own validated layout if
it needs to address a particular device.

## Scenes, layers, masks, and blending

A scene stores layers from bottom to top. Each layer has a stable ID, enabled
flag, opacity from 0 to 1, optional key mask, blend mode, and effect. `mask: null`
selects every key. An empty mask selects no keys. A mask includes LEDs whose
`key_id` is in the set; keyless LEDs are excluded when a mask is present.

Disabled layers, masked-out LEDs, opacity zero, and effects with no active sample
are transparent. They leave lower layers unchanged. Layer opacity multiplies
the sampled alpha, so an alpha-zero sample remains transparent in every blend
mode.

The engine starts each LED at black and applies these formulas independently to
the three RGB channels, where `d` is the current channel, `s` is the sampled
channel, and `c = sample_alpha × layer_opacity`:

- `over`: `s × c + d × (1 − c)` (source-over on the opaque RGB output)
- `add`: `min(1, d + s × c)`
- `multiply`: `d × (1 − c + s × c)`

Channels are clamped to `[0, 1]`, multiplied by 255, and rounded to the nearest
byte. RGBA samples are useful for fades and blend coverage; frames contain RGB
because that is the output needed by keyboard-lighting transports.

The validator accepts at most 32 layers per scene. Blend order is significant:
an upper layer sees the already-composed result below it.

## Effects and deterministic time

`Engine::render_at(time_ms)` uses only the supplied absolute `u64` millisecond
timestamp and retained key events. It does not read wall time. Repeating a call
with the same engine state and timestamp returns the same frame, and seeking to
an earlier time is supported while the needed events remain in history.

- **Static** returns its configured RGBA color at every time.
- **Breathing** pulses one color with `0.5 − 0.5 × cos(2π × phase)`, where
  `phase = (time_ms mod period_ms) / period_ms`. It is dark at phase 0 and full
  strength halfway through the period.
- **Spectrum** cycles full-saturation HSV hue over the configured period. Its
  wavelength shifts the hue by horizontal position, so LEDs at different `x`
  coordinates can show different parts of the spectrum at the same time.
- **Wave** repeats a configured color gradient over a spatial wavelength. Its
  phase is `(coordinate − speed × time_seconds) / wavelength`; the coordinate
  can be horizontal, vertical, or diagonal. Negative speed reverses direction.
- **Reactive** lights only the LED mapped to a pressed key. It fades linearly
  from full strength to transparent over `fade_ms`; at exactly that age it is
  inactive. If several retained presses target the same key, the newest press
  at or before render time determines the fade.
- **Ripple** starts at a pressed key's key coordinate and travels by Euclidean
  distance in matrix cells. `speed_cells_per_second` sets the ring radius,
  `width_cells` is the full ring thickness (coverage falls linearly to zero at
  half that distance from the radius), and the configured color gradient follows
  progress from the origin to the radius at the end of the duration.
  Its age must be less than `duration_ms`; otherwise it is transparent. When
  rings overlap, the sample with the strongest ring coverage is used.

Periods, fades, and ripple duration must be from 1 ms through one year. Colors
and opacity must be finite and within `[0, 1]`. Wavelengths, speeds, gradient
sizes, and ripple width have explicit validation bounds; zero periods, zero
ripple speed, NaN, infinity, and out-of-range values are errors. No effect is a
name-only placeholder: every listed effect produces a time- or position-based
sample.

## Event history and seeking

Call `key_press(key_id, time_ms)` when a key press occurs. IDs must exist in the
current layout, and event timestamps must be nondecreasing; equal timestamps
are allowed and retain insertion order. Out-of-order input is rejected without
changing history. Events later than a render timestamp are ignored for that
render.

The default event history holds 256 events. `Engine::with_event_capacity` lets
the caller choose a capacity from 1 through 512. At capacity, a newly accepted
event evicts the oldest event. Expired events are not silently removed during
rendering; they remain until capacity eviction or a successful layout/config
replacement. The capacity bounds memory, and it also defines how far back
reactive effects can be reconstructed after a seek: evicted event history is
unavailable. Rendering itself never mutates this history.

Replacing a scene validates every layer before commit and preserves events,
since the layout remains the same. Replacing a layout validates it against the
active scene, then clears history after a successful commit because event IDs
and coordinates belong to the prior layout. If either validation fails, state
and history remain unchanged. `set_configuration(layout, scene)` and the
`set_configuration` JSON command validate both objects together, then replace
both atomically and clear old events. This supports changing a device layout and
its masks in one operation.

## Rust API

Create an `Engine` with a validated `Layout` and `Scene`, then call
`render_at(time_ms)`. `set_scene`, `set_layout`, and `set_configuration` are
validated atomic operations. `execute(Command)` and `handle_request(Request)`
expose the same operations through typed serializable commands. The direct Rust
example demonstrates building a layout, composing layers, recording a press,
and reading the RGB matrix.

```rust
use chroma::{Effect, Engine, KeyId, KeyPosition, Layout, LedPosition, Layer, Rgba, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = KeyId::from("key-a");
    let layout = Layout::new(
        "my-layout",
        1,
        1,
        vec![KeyPosition::new(key.clone(), 0, 0)],
        vec![LedPosition::new("led-a", Some(key), 0, 0)],
    );
    let scene = Scene::new(
        "my-scene",
        vec![Layer::new("base", Effect::Static { color: Rgba::rgb(0.2, 0.4, 1.0) })],
    );
    let engine = Engine::new(layout, scene)?;
    let frame = engine.render_at(0);
    assert_eq!(frame.cell_at(0, 0).unwrap().rgb.r, 51);
    Ok(())
}
```

## JSON-lines protocol

The `json_lines` example starts with an empty scene and the synthetic 6×22
geometry template. It accepts one UTF-8 JSON request per line and writes exactly
one JSON response line for each input line. Each input line is limited to 1 MiB;
an overlong line is discarded through its newline and returns an error without
being buffered in full. The example is a process adapter for CLI or future IPC
wrappers; it does not run an HTTP service or device adapter.

Every request has `version: 1` and a typed command. Example:

```json
{"version":1,"command":{"type":"render_at","time_ms":1000}}
```

Other command payloads are `set_layout` with `layout`, `set_scene` with
`scene`, `set_configuration` with both `layout` and `scene`, and `key_press`
with `key_id` and `time_ms`. A static scene request looks like:

```json
{"version":1,"command":{"type":"set_scene","scene":{"id":"desk","layers":[{"id":"base","enabled":true,"opacity":1.0,"mask":null,"blend":"over","effect":{"type":"static","color":{"r":0.0,"g":0.1,"b":0.4,"a":1.0}}}]}}}
```

Successful responses use `{ "version": 1, "ok": true, "result": ... }`.
Errors use `{ "version": 1, "ok": false, "error": { "code": ..., "message": ... } }`.
Error codes identify protocol-version, JSON, validation, layout, event, or
parameter failures. A rejected command leaves engine state unchanged. Input
structs reject unknown fields so misspelled parameters do not silently take
defaults. The request and response types can be serialized and deserialized by
Rust clients.

## Optional consumers and device adapters

An application can supply a `Layout`, optionally call `key_press` from its own
input source, call `render_at` from a client-owned scheduler, and consume each
non-hole `FrameCell` by stable LED ID. The consumer owns its UI, process
lifecycle, and storage. A separate device adapter would own device discovery,
capabilities, pacing, reconnection, and transport errors.

OpenRazer may be one possible adapter for some Razer devices, and an
Omarchy/Quickshell UI may be one possible client. Neither is implemented or
required by this crate. Other manufacturers, such as HyperX, would likewise
need their own separate adapter; this documentation does not claim their
devices are supported.
