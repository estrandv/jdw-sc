
/*
    OSC structs for careful parsing and management of expected message and bundle types.
 */

use std::option::Option;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use jdw_osc_lib::model::{OscArgHandler, TaggedBundle, TimedOSCPacket};
use log::{info, warn};
use rosc::{OscMessage, OscPacket, OscType};

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
    pub gate_time: BigDecimal, // TODO: Should be in pre-calculated ms rather than beats; this application has no BPM. Same for all time args, really. hard. 
    pub args: Vec<OscType> // Named args such as "bus" or "rel"
}


impl NoteOnTimedMessage {

    pub fn new(msg: &OscMessage) -> Result<NoteOnTimedMessage, String> {

        msg.expect_addr("/note_on_timed")?;
        msg.expect_args(3)?;

        let synth_name = msg.get_string_at(0, "synth name")?;
        let external_id = msg.get_string_at(1, "external id")?;
        let gate_time_str = msg.get_string_at(2, "gate time")?;
        let gate_time = BigDecimal::from_str(&gate_time_str).unwrap();

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

        info!("Began parsing an nrt_record bundle with content size: {}", message_bundle.content.len());

        let timed_messages: Vec<_> = message_bundle.content.iter()
            .map(|packet| return match packet {
                OscPacket::Bundle(bun) => {
                    let tagged = TaggedBundle::new(&bun)?;
                    info!("Parsing tagged bundle for NRT! {}", tagged.bundle_tag);
                    Ok(TimedOSCPacket::from_bundle(tagged)?)
                }
                _ => {
                    warn!("Unexpected non-bundle when unpacking timed messages bundle content");
                    Err("Unexpected non-bundle when unpacking timed messages bundle".to_string())
                }
            })
            // TODO: Pref I guess we want to error check properly but cba right now
            //  - at least this gives a message on crash
            .map(|m| m.unwrap())
            .collect();

        info!("Timed messages length: {}", timed_messages.len());

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