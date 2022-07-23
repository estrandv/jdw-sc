
/*
    OSC structs for careful parsing and management of expected message and bundle types.
 */

use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use rosc::{OscBundle, OscError, OscMessage, OscPacket, OscType};
use std::option::Option;
use std::sync::{Arc, Mutex};
use log::{debug, warn};
use crate::SampleDict;

/*
    Adding some convenience functions for OscMessage args
 */
trait OscArgHandler {
    fn expect_addr(&self, addr_name: &str) -> Result<(), String>;
    fn expect_args(&self, amount: usize) -> Result<String, String>;
    fn get_string_at(&self, index: usize, name: &str, ) -> Result<String, String>;
    fn get_float_at(&self, index: usize, name: &str, ) -> Result<f32, String>;
    fn get_int_at(&self, index: usize, name: &str, ) -> Result<i32, String>;
}

impl OscArgHandler for OscMessage {

    fn expect_addr(&self, addr_name: &str) -> Result<(), String> {
        if &self.addr.to_string() != addr_name {
            return Err(format!("Attempted to format {} as the wrong kind of message - this likely a human error in the source code", addr_name));
        }

        Ok(())
    }

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

// Verify that custom args follow the String,float,String,float... pattern
// Note: This could possibly be a bit expensive time-wise!
fn validate_args(args: &Vec<OscType>) -> Result<(), String> {

    let mut next_is_string = true;

    for arg in args {
        match arg {
            OscType::Float(_) => {
                if next_is_string {
                    return Err("Malformed message: Custom arg float where string expected".to_string());
                }

                next_is_string = true;
            },
            OscType::String(_) => {
                if !next_is_string {
                    return Err("Malformed message: Custom arg string where float expected".to_string());
                }

                next_is_string = false;
            },
            _ => {
                return Err("Malformed message: Custom arg in message not of type string or float".to_string());
            }
        }
    }

    Ok(())
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
    pub fn new(msg: &OscMessage) -> Result<NoteOnTimedMessage, String> {

        msg.expect_addr("/note_on_timed")?;
        msg.expect_args(3)?;

        let synth_name = msg.get_string_at(0, "synth name")?;
        let external_id = msg.get_string_at(1, "external id")?;
        let gate_time = msg.get_float_at(2, "gate time")?;

        let named_args = if msg.args.len() > 3 {(&msg.args[3..].to_vec()).clone()} else {vec![]};

        validate_args(&named_args)?;

        // TODO: Ensure even number of named args and that they conform to str,double pattern

        Ok(NoteOnTimedMessage {
            synth_name,
            external_id,
            gate_time,
            args: named_args
        })
    }

}

// ProscNoteCreateMessage
// Non-timed regular s_new with external_id for later modifications
pub struct NoteOnMessage {
    pub synth_name: String, // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub args: Vec<OscType>, // Named args such as "bus" or "rel"
}

impl NoteOnMessage {
    pub fn new (msg: &OscMessage) -> Result<NoteOnMessage, String> {
        msg.expect_args(2)?;

        let synth_name = msg.get_string_at(0, "synth name")?;
        let external_id = msg.get_string_at(1, "external id")?;

        let named_args = if msg.args.len() > 2 {(&msg.args[2..].to_vec()).clone()} else {vec![]};
        validate_args(&named_args)?;

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
    pub fn new(message: &OscMessage) -> Result<NoteModifyMessage, String> {

        message.expect_args(2)?;

        let external_id_regex = message.get_string_at(0, "external id regex")?;
        let args = if message.args.len() > 1 {(&message.args[1..].to_vec()).clone()} else {vec![]};
        validate_args(&args)?;

        Ok(NoteModifyMessage {
            external_id_regex,
            args
        })

    }

}

// Example below of args in order with "" as category (= Empty)
// ["/play_sample", "my_unique_id", "example", 2, "", "arg1", 0.2, "arg2", 0.4, ...]
pub struct PlaySampleMessage {
    pub external_id: String,
    pub sample_pack: String, // The parent dir of the sample file
    pub index: usize, // Sample number - either as plain order in dir or in a given category
    pub category: Option<String>, // TODO: Arbitrary string codes... is there a better way?
    pub args: Vec<OscType>, // Args to set (same as in SNewTimedGateMessage)
}

impl PlaySampleMessage {
    pub fn new(message: &OscMessage) -> Result<PlaySampleMessage, String> {

        message.expect_args(4)?;

        let external_id = message.get_string_at(0, "external_id")?;
        let sample_pack = message.get_string_at(1, "sample_pack")?;
        let index = message.get_int_at(2, "index")?;

        if index < 0 {
            return Err("Index arg in sample message incompatible: negative".to_string());
        }

        let cat_arg = message.get_string_at(3, "category")?;
        let category = if cat_arg == "".to_string() {None} else {Some(cat_arg)};
        let args = if message.args.len() > 4 {(&message.args[4..].to_vec()).clone()} else {vec![]};
        validate_args(&args)?;

        Ok(PlaySampleMessage {
            external_id,
            sample_pack,
            index: index as usize,
            category,
            args
        })

    }


}

/*
    In order to properly utilize bundles I have created a standard where the first
        packet in every JDW-compatible bundle is an OSC message with a bundle type
        string contained within, e.g.: ["/bundle_tag", "nrt_record_request"]
 */
#[derive(Debug)]
pub struct TaggedBundle {
    pub bundle_tag: String,
    pub contents: Vec<OscPacket>
}

impl TaggedBundle {
    pub fn new(bundle: &OscBundle) -> Result<TaggedBundle, String> {
        let first_msg = match bundle.content.get(0).ok_or("Empty bundle")?.clone() {
            OscPacket::Message(msg) => { Option::Some(msg) }
            OscPacket::Bundle(_) => {Option::None}
        }.ok_or("First element in bundle not an info message!")?;

        if first_msg.addr != "/bundle_info" {
            return Err(format!("Expected /bundle_info as first message in bundle, got: {}", &first_msg.addr));
        }

        let bundle_tag = first_msg.args.get(0)
            .ok_or("bundle info empty")?
            .clone()
            .string().ok_or("bundle info should be a string")?;

        let contents = if bundle.content.len() > 1 {(&bundle.content[1..].to_vec()).clone()} else {vec![]};

        debug!("Tagged bundle: {}::{:?}", &bundle_tag, contents.clone());

        Ok(TaggedBundle {
            bundle_tag,
            contents
        })
    }

    fn get_packet(&self, content_index: usize) -> Result<OscPacket, String> {
        self.contents.get(content_index)
            .map(|pct| pct.clone())
            .ok_or("Failed to fetch packet".to_string())
    }

    fn get_message(&self, content_index: usize) -> Result<OscMessage, String> {
        self.contents.get(content_index)
            .map(|pct| pct.clone())
            .ok_or("Invalid index".to_string())
            .map(|pct| match pct {
                OscPacket::Message(msg) => {
                    Ok(msg)
                }
                _ => {Err("Not a message".to_string())}
            })
            .flatten()
    }

    fn get_bundle(&self, content_index: usize) -> Result<OscBundle, String> {
        self.contents.get(content_index)
            .map(|pct| pct.clone())
            .ok_or("Invalid index".to_string())
            .map(|pct| match pct {
                OscPacket::Bundle(msg) => {
                    Ok(msg)
                }
                _ => {Err("Not a bundle".to_string())}
            })
            .flatten()
    }
}

// TODO: The sooneer we make PACKET the contained class, the better 
/*
    Timed osc packets are used to delay execution. This has uses both for NRT recording as
        well as sequencer spacing or timed gate off messages.
    [/bundle_info, "timed_msg"]
    [/timed_msg_info, 0.0]
    [... packet ...]
 */
#[derive(Debug, Clone)]
pub struct TimedOSCPacket {
    pub time: f32,
    pub message: OscMessage, // TODO: Stepping stone, delete
    pub packet: OscPacket,
}

impl TimedOSCPacket {

    pub fn from_bundle(bundle: TaggedBundle) -> Result<TimedOSCPacket, String>{
        if &bundle.bundle_tag != "timed_msg" {
            return Err(format!("Attempted to parse {} as timed_msg bundle", &bundle.bundle_tag));
        }

        let info_msg = bundle.get_message(0)?;
        let actual_msg = bundle.get_message(1)?;
        let packet = bundle.get_packet(1)?;

        info_msg.expect_addr("/timed_msg_info")?;
        let time = info_msg.get_float_at(0, "time")?;

        Ok(TimedOSCPacket {
            time,
            message: actual_msg,
            packet
        })

    }
}

/*
    Extracted from a bundle:
    [/bundle_info, "nrt_record"]
    [/nrt_record_info, <bpm: 120.0>, <file_name: "myfile.wav">, <end_beat: 44.0>]
    followed by untagged bundle: all contained timed messages
 */
pub struct NRTRecordMessage {
    pub file_name: String,
    pub bpm: f32,
    pub messages: Vec<TimedOSCPacket>,
    pub end_beat: f32
}

impl NRTRecordMessage {
    pub fn from_bundle(bundle: TaggedBundle) -> Result<NRTRecordMessage, String>{
        if &bundle.bundle_tag != "nrt_record" {
            return Err(format!("Attempted to parse {} as nrt_record bundle", &bundle.bundle_tag));
        }

        let info_msg = bundle.get_message(0)?;
        let message_bundle = bundle.get_bundle(1)?;

        let timed_messages: Vec<_> = message_bundle.content.iter()
            .map(|packet| return match packet {
                OscPacket::Bundle(bun) => {
                    let tagged = TaggedBundle::new(&bun)?;
                    Ok(TimedOSCPacket::from_bundle(tagged)?)
                }
                _ => { Err("Unexpected non-bundle when unpacking timed messages bundle".to_string()) }
            })
            // TODO: Pref I guess we want to error check properly but cba right now
            .filter(|m| m.is_ok())
            .map(|m| m.unwrap())
            .collect();

        info_msg.expect_addr("/nrt_record_info")?;
        let bpm = info_msg.get_float_at(0, "bpm")?;
        let file_name = info_msg.get_string_at(1, "file_name")?;
        let end_beat = info_msg.get_float_at(2, "end_beat")?;

        Ok(NRTRecordMessage {
            file_name,
            bpm,
            messages: timed_messages,
            end_beat
        })

    }
}