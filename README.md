# SuperCollider Wrapper (jdw-sc)

Manages a running SuperCollider instance (sclang + scsynth) for the JackDAW system. Provides an OSC bridge with enhanced functionality beyond raw scsynth control.

## Features

- **Full sclang/scsynth lifecycle management** — start, monitor, graceful shutdown
- **Custom OSC messages** extending raw scsynth protocol:
  - `/note_on_timed` — note-on with automatic note-off after duration
  - `/note_modify` — change running synth parameters by external ID (no need to track server-assigned node IDs)
  - `/load_scd` — load SynthDef strings without restarting sclang
  - `/play_sample` / `/load_sample` — sample playback with pack/category organization
  - `/nrt_record` — non-real-time recording support
- **Node ID registry** — maps external IDs to scsynth node IDs, enabling modifier commands without knowing the server's internal ID
- **`{nodeId}` templating** — placeholder replaced with the actual scsynth node ID for unique external identifiers
- **`/init` handshake** — waits for sclang to signal readiness before accepting commands

## Architecture

```
OSC Clients  <--->  jdw-sc  <--->  scsynth (audio)
                         |
                    sclang (SynthDef compilation)
```

## Dependencies

- `rosc` — OSC encoding/decoding
- `jdw-osc-lib` — shared OSC protocol library
- `serde`/`serde_json` — configuration serialization
- `regex` — `{nodeId}` template replacement

## Version

Tested with SuperCollider 3.13.0.

## Important Caveats

- `buf` arg in sampler SynthDefs is internally managed — avoid supplying it manually
- `gate` arg is the universal note-off signal — SynthDefs without gate logic won't respond to note-offs
- Insufficient lifecycle management: crashes/exits may leave orphaned sclang/scsynth processes
