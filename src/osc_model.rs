
// TODO: Structs mainly for standard incoming OSC messages and bundles
// Idea is to parse it straight into usable data
// We can probably re-use the old zmq messages via manual parsing instead of named json

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use rosc::{OscError, OscMessage, OscType};
use serde::de::Unexpected::Option;

// Re-implementation of JdwPlayNoteMsg
// Initial structure below: (Note that we might want to expose other s_new args eventually)
// ["/s_new_timed_gate", "my_synth", "kb_my_synth_n33", 0.2, "arg1", 0.2, "arg2", 0.4, ...]
pub struct SNewTimedGateMessage {
    pub synth_name: String, // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub gate_time: f32, // Should be in ms rather than beats; wrapper has no BPM.
    pub args: Vec<OscType> // Named args such as "bus" or "rel"
}

impl SNewTimedGateMessage {
    pub fn new(msg: OscMessage) -> Result<SNewTimedGateMessage, String> {
        if msg.addr != "/s_new_timed_gate" {
            Err(format!("Attempted to parse {} as s_new_timed_gate", msg.addr))
        } else {

            if msg.args.len() < 3 {
                return Err("Message did not contain the 2 first required args.".to_string());
            }

            // get index -> cast as string -> map option to result -> assign or throw
            let synth_name = msg.args
                .get(0)
                .map_or(None, |some| some.clone().string())
                .map_or(Err("synth_name not found"), |s| Ok(s))?;

            let external_id = msg.args
                .get(1)
                .map_or(None, |some| some.clone().string())
                .map_or(Err("external_id not found"), |s| Ok(s))?;

            let gate_time = msg.args
                .get(2)
                .map_or(None, |some| some.clone().float())
                .map_or(Err("gate_time not found"), |s| Ok(s))?;

            // TODO: Not sure about start index of slice here
            let named_args = if msg.args.len() > 3 {(&msg.args[2..].to_vec()).clone()} else {vec![]};

            // TODO: Ensure even number of named args and that they conform to str,double pattern

            // TODO: What about the actual OSC we then send to server? Construct now or later? I guess
            //  it isn't a big difference since it will be sent immediately on receive either way...
            Ok(SNewTimedGateMessage {
                synth_name,
                external_id,
                gate_time,
                args: named_args
            })
        }
    }

    // TODO: Note differences with supercollider.rs - typically you need access to e.g. running notes
    // in order to construct the actual osc args for s_new or n_set (since they are indices).
    // It is probably best to keep the old parts intact and use the data from these messages as base for func calls.

}

// ProscNoteCreateMessage
// Non-timed regular s_new with external_id for later modifications
pub struct SNewTaggedMessage {
    pub synth_name: String, // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub args: HashMap<String, f32>, // Named args such as "bus" or "rel"
}

// ProscNoteModifyMessage
// n_set implementation with added external_id to allow modifying any note
// TODO: Should external_id be wildcard?
// NOTE: Note-off doesn't need its own message; it is simply an n_set with gate=0
pub struct NSetTaggedMessage {
    pub external_id: String, // External id of note to change args for
    pub args: HashMap<String, f32>, // Args to set
}