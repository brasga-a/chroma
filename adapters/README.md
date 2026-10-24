# Hardware adapters

Hardware communicators belong here. Each adapter consumes Chroma RGB frames
and translates stable LED IDs and colors into its own device protocol or API.

Adapters own discovery, capability checks, physical LED mapping, frame pacing,
reconnection, and transport errors. They depend on the engine contract; the
engine does not depend on an adapter, manufacturer, or hardware library.

No hardware adapter is implemented here yet.
