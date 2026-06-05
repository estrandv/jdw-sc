# AGENTS.md — jdw-sc

## Source Structure

```
src/
  main.rs                 # Entry: init, lifecycle, OSC daemon startup
  osc_model.rs            # OSC message types: NoteOnTimed, NoteOn, NoteModify, PlaySample, etc.
  sc_process_management.rs # SuperCollider process lifecycle (spawn, monitor, kill)
  osc_daemon.rs           # OSC receive/send loop and dispatcher
  node_lookup.rs          # External ID -> scsynth node ID registry
  nrt_record.rs           # Non-real-time recording orchestration
  sampling.rs             # Sample management (packs, categories, playback)
  scd_templating.rs       # {nodeId} template replacement
  config.rs               # Port configuration
python/                   # Example scripts for all custom messages
```

## Key Message Types

- `NoteOnTimed` — `{ synth_name, external_id, args, gate_time_ms }` — auto note-off after gate_time
- `NoteModify` — `{ external_id, args }` — change params of running synth
- `PlaySample` — `{ pack, category, sample_name, args }` — trigger sample playback
- `LoadSample` — `{ pack, category, file_path }` — load sample into buffer
- `NRTRecord` — `{ duration_ms, output_path }` — offline render
- `RealTimePacket` — raw packet to forward to scsynth

## Custom Protocol

All custom messages are sent via OSC to jdw-sc's control port. Non-custom messages are forwarded directly to scsynth.

- `/note_on_timed` — float args: [synth_name, external_id, duration_ms, args...]
- `/note_modify` — float args: [external_id, args...]
- `/load_scd` — string arg: [scd_source_code]
- `/nrt_record` — float: [duration_ms], string: [output_path]

## Node Registry

- On `/note_on`, external_id + returned scsynth node_id are stored in a HashMap
- `/note_modify` looks up external_id, retrieves node_id, sends `/n_set` to scsynth
- On `/n_end` from scsynth, the entry is removed

## Build & Run

```bash
cargo build --release
cargo run --release
```

## Tested With

SuperCollider 3.13.0
