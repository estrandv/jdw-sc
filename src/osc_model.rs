
// TODO: Structs mainly for standard incoming OSC messages and bundles
// Idea is to parse it straight into usable data
// We can probably re-use the old zmq messages via manual parsing instead of named json

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use rosc::{OscError, OscMessage, OscType};
use std::option::Option;

/*
    Adding some convenience functions for OscMessage args
 */
trait OscArgHandler {
    fn expect_args(&self, amount: usize) -> Result<String, String>;
    fn get_string_at(&self, index: usize, name: &str, ) -> Result<String, String>;
    fn get_float_at(&self, index: usize, name: &str, ) -> Result<f32, String>;
}

impl OscArgHandler for OscMessage {

    fn expect_args(&self, amount: usize) -> Result<String, String> {

        if self.args.len() < (amount + 1) {
            return Err(format!("Message did not contain the {} first required args.", amount));
        }

        Ok("Ok".to_string())
    }

    fn get_string_at(&self, index: usize, name: &str, ) -> Result<String, String> {
        let err_msg = format!("{} string not found as {}th arg", name, index);
        self.args
            .get(index)
            .map_or(None, |some| some.clone().string())
            .map_or(Err(err_msg), |s| Ok(s))
    }

    fn get_float_at(&self, index: usize, name: &str, ) -> Result<f32, String> {
        let err_msg = format!("{} float not found as {}th arg", name, index);
        self.args
            .get(index)
            .map_or(None, |some| some.clone().float())
            .map_or(Err(err_msg), |s| Ok(s))
    }

}

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

            msg.expect_args(2)?;

            let synth_name = msg.get_string_at(0, "synth name")?;
            let external_id = msg.get_string_at(1, "external id")?;
            let gate_time = msg.get_float_at(2, "gate time")?;

            let named_args = if msg.args.len() > 3 {(&msg.args[3..].to_vec()).clone()} else {vec![]};

            // TODO: Ensure even number of named args and that they conform to str,double pattern

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
    pub args: Vec<OscType>, // Named args such as "bus" or "rel"
}

// ProscNoteModifyMessage
// n_set implementation with added external_id to allow modifying any note
// TODO: Should external_id be wildcard?
// NOTE: Note-off doesn't need its own message; it is simply an n_set with gate=0
pub struct NSetTaggedMessage {
    pub external_id: String, // External id of note to change args for
    pub args: Vec<OscType>, // Args to set (same as in SNewTimedGateMessage)
}

// Example below of args in order with "" as category (= Empty)
// ["/play_sample", "example", 2, "", "arg1", 0.2, "arg2", 0.4, ...]
pub struct PlaySampleMessage {
    pub sample_pack: String, // The parent dir of the sample file
    pub index: i32, // Sample number - either as plain order in dir or in a given category
    pub category: Option<String>, // TODO: Arbitrary string codes... is there a better way?
    pub args: Vec<OscType>, // Args to set (same as in SNewTimedGateMessage)
}

impl PlaySampleMessage {
    pub fn new(message: OscMessage) -> Result<PlaySampleMessage, String> {
        Err("unimpl".to_string())
    }
}