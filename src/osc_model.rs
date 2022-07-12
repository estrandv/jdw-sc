
// TODO: Structs mainly for standard incoming OSC messages and bundles
// Idea is to parse it straight into usable data
// We can probably re-use the old zmq messages via manual parsing instead of named json

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use rosc::{OscError, OscMessage, OscType};
use std::option::Option;
use std::sync::{Arc, Mutex};
use crate::SampleDict;

/*
    Adding some convenience functions for OscMessage args
 */
trait OscArgHandler {
    fn expect_args(&self, amount: usize) -> Result<String, String>;
    fn get_string_at(&self, index: usize, name: &str, ) -> Result<String, String>;
    fn get_float_at(&self, index: usize, name: &str, ) -> Result<f32, String>;
    fn get_int_at(&self, index: usize, name: &str, ) -> Result<i32, String>;
}

impl OscArgHandler for OscMessage {

    fn expect_args(&self, amount: usize) -> Result<String, String> {

        if self.args.len() < amount {
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

    fn get_int_at(&self, index: usize, name: &str, ) -> Result<i32, String> {
        let err_msg = format!("{} float not found as {}th arg", name, index);
        self.args
            .get(index)
            .map_or(None, |some| some.clone().int())
            .map_or(Err(err_msg), |s| Ok(s))
    }

}

// Initial structure below: (Note that we might want to expose other s_new args eventually)
// ["/note_on_timed", "my_synth", "kb_my_synth_n33", 0.2, "arg1", 0.2, "arg2", 0.4, ...]
pub struct NoteOnTimedMessage {
    pub synth_name: String, // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub gate_time: f32, // Should be in ms rather than beats; wrapper has no BPM.
    pub args: Vec<OscType> // Named args such as "bus" or "rel"
}

impl NoteOnTimedMessage {
    pub fn new(msg: OscMessage) -> Result<NoteOnTimedMessage, String> {
        if msg.addr != "/note_on_timed" {
            Err(format!("Attempted to parse {} as note_on_timed", msg.addr))
        } else {

            msg.expect_args(3)?;

            let synth_name = msg.get_string_at(0, "synth name")?;
            let external_id = msg.get_string_at(1, "external id")?;
            let gate_time = msg.get_float_at(2, "gate time")?;

            let named_args = if msg.args.len() > 3 {(&msg.args[3..].to_vec()).clone()} else {vec![]};

            // TODO: Ensure even number of named args and that they conform to str,double pattern

            Ok(NoteOnTimedMessage {
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
pub struct NoteOnMessage {
    pub synth_name: String, // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub args: Vec<OscType>, // Named args such as "bus" or "rel"
}

impl NoteOnMessage {
    pub fn new (msg: OscMessage) -> Result<NoteOnMessage, String> {
        msg.expect_args(2)?;

        let synth_name = msg.get_string_at(0, "synth name")?;
        let external_id = msg.get_string_at(1, "external id")?;

        let named_args = if msg.args.len() > 2 {(&msg.args[2..].to_vec()).clone()} else {vec![]};

        // TODO: Ensure even number of named args and that they conform to str,double pattern

        Ok(NoteOnMessage {
            synth_name,
            external_id,
            args: named_args
        })
    }
}

// ProscNoteModifyMessage
// n_set implementation with added external_id to allow modifying any note
// NOTE: Note-off doesn't need its own message; it is simply an n_set with gate=0
pub struct NoteModifyMessage {
    pub external_id_regex: String, // Modify all running external ids matching this regex
    pub args: Vec<OscType>, // Args to set (same as in SNewTimedGateMessage)
}

impl NoteModifyMessage {
    pub fn new(message: OscMessage) -> Result<NoteModifyMessage, String> {

        message.expect_args(2)?;

        let external_id_regex = message.get_string_at(0, "external id regex")?;
        let args = if message.args.len() > 1 {(&message.args[1..].to_vec()).clone()} else {vec![]};

        Ok(NoteModifyMessage {
            external_id_regex,
            args
        })

    }

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

        message.expect_args(3)?;

        let sample_pack = message.get_string_at(0, "sample_pack")?;
        let index = message.get_int_at(1, "index")?;
        let cat_arg = message.get_string_at(2, "category")?;
        let category = if cat_arg == "".to_string() {None} else {Some(cat_arg)};
        let args = if message.args.len() > 3 {(&message.args[3..].to_vec()).clone()} else {vec![]};

        Ok(PlaySampleMessage {
            sample_pack,
            index,
            category,
            args
        })

    }

    // TODO: A bit unhappy with having to use a SampleDict in this strictly OSC library
    // Once we start porting osc_model to other projects we should port this impl to
    // a separate rs file
    pub fn get_args_with_buf(&self, samples: Arc<Mutex<SampleDict>>) -> Vec<OscType> {
        let mut base_args = self.args.clone();

        let buf_nr = samples
            .lock()
            .unwrap()
            .get_buffer_number(&self.sample_pack, self.index, self.category.clone())
            .unwrap_or(0); // Should probably be some kind of error, but for now default to base buf

        // TODO: Buf might already be in it. Might be good to wipe it.
        base_args.push(OscType::String("buf".to_string()));
        base_args.push(OscType::Int(buf_nr));

        base_args
    }
}