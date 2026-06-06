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

## Node Registry (`node_lookup.rs`)

- On `/note_on_timed` / `/note_on` / `/play_sample`, external_id + assigned node_id are stored in `HashMap<String, i32>`
- `/note_modify` does a regex match against external_ids, retrieves node_ids, sends `/n_set` to scsynth
- `/free_notes` does a regex match, sends `/n_free` to scsynth for each match, AND removes entries from registry
- **There is NO `/n_end` handler** — scsynth never notifies jdw-sc when a note ends. The `clear()` method exists but is `#[allow(dead_code)]` (never called).
- **`{nodeId}` template**: if an external_id contains the literal string `{nodeId}`, it gets replaced with the assigned scsynth node_id before storage. This is the main uniqueness mechanism — since `curr_id` starts at 100 and monotonically increments, every `{nodeId}` substitution produces a globally unique value, even across loop boundaries.
- `curr_id` starts at 100, increments by 1 per `create_node_id` call. Never resets.
- **Bottom line**: external IDs must be unique per-call or the note is rejected. The `{nodeId}` template in Python's `ElementConverter` ensures this by embedding the unique scsynth node ID.

## Build & Run

```bash
cargo build --release
cargo run --release
```

## Tested With

SuperCollider 3.13.0
