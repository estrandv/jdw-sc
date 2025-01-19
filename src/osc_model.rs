/*
   OSC structs for careful parsing and management of expected message and bundle types.
*/

use std::{convert::TryFrom, time::SystemTime};

use bigdecimal::BigDecimal;
use jdw_osc_lib::model::{OscArgHandler, TaggedBundle, TimedOSCPacket};
use log::{info, warn};
use rosc::{OscMessage, OscPacket, OscType};

pub struct RealTimePacket {
    pub packet: OscPacket,
    pub time: SystemTime,
}

impl RealTimePacket {
    pub fn new(tagged_bundle: TaggedBundle) -> Result<RealTimePacket, String> {
        let info_packet = tagged_bundle.contents.get(0).ok_or("No packets")?;
        let actual_packet = tagged_bundle
            .contents
            .get(1)
            .ok_or("Missing actual packet")?
            .clone();
        if let OscPacket::Message(info_msg) = info_packet {
            let timestamp = info_msg
                .clone()
                .args
                .clone()
                .get(0)
                .ok_or("No info msg arg")?
                .clone()
                .time()
                .ok_or("Malformed time arg")?;

            let time =
                SystemTime::try_from(timestamp).map_err(|_| "Malformed timestamp".to_string())?;

            Ok(RealTimePacket {
                packet: actual_packet,
                time,
            })
        } else {
            Err("Info message was a bundle".to_string())
        }
    }
}

// Initial structure below: (Note that we might want to expose other s_new args eventually)
// ["/note_on_timed", "my_synth", "kb_my_synth_n33", 0.2, "arg1", 0.2, "arg2", 0.4, ...]
pub struct NoteOnTimedMessage {
    pub synth_name: String,  // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub gate_time: BigDecimal,
    pub delay_ms: u64,
    pub args: Vec<OscType>, // Named args such as "bus" or "rel"
}

impl NoteOnTimedMessage {
    pub fn new(msg: &OscMessage) -> Result<NoteOnTimedMessage, String> {
        msg.expect_addr("/note_on_timed")?;
        msg.expect_args(4)?;

        let synth_name = msg.get_string_at(0, "synth name")?;
        let external_id = msg.get_string_at(1, "external id")?;
        let gate_time = msg.get_bigdecimal_at(2, "gate time")?;
        let delay_ms = msg.get_u64_at(3, "delay_ms")?;
        let named_args = msg.get_varargs(4)?;

        Ok(NoteOnTimedMessage {
            synth_name,
            external_id,
            gate_time,
            delay_ms,
            args: named_args,
        })
    }
}

#[derive(Clone)]
pub struct LoadSampleMessage {
    pub file_path: String,
    pub sample_pack: String,
    pub buffer_number: i32,
    pub category_tag: String,
    pub tone_index: i32,
}

impl LoadSampleMessage {
    pub fn new(msg: &OscMessage) -> Result<LoadSampleMessage, String> {
        msg.expect_addr("/load_sample")?;
        msg.expect_args(5)?;

        Ok(LoadSampleMessage {
            file_path: msg.get_string_at(0, "file_path")?,
            sample_pack: msg.get_string_at(1, "sample_pack")?,
            buffer_number: msg.get_int_at(2, "buffer_number")?,
            category_tag: msg.get_string_at(3, "category_tag")?,
            tone_index: msg.get_int_at(4, "tone_index")?,
        })
    }
}

// ProscNoteCreateMessage
// Non-timed regular s_new with external_id for later modifications
pub struct NoteOnMessage {
    pub synth_name: String,  // The synth upon which to play the note.
    pub external_id: String, // Identifier for note to allow later modification.
    pub delay_ms: u64,
    pub args: Vec<OscType>, // Named args such as "bus" or "rel"
}

impl NoteOnMessage {
    pub fn new(msg: &OscMessage) -> Result<NoteOnMessage, String> {
        msg.expect_addr("/note_on")?;
        msg.expect_args(3)?;

        let synth_name = msg.get_string_at(0, "synth name")?;
        let external_id = msg.get_string_at(1, "external id")?;
        let delay_ms = msg.get_u64_at(2, "delay_ms")?;
        let named_args = msg.get_varargs(3)?;

        Ok(NoteOnMessage {
            synth_name,
            external_id,
            delay_ms,
            args: named_args,
        })
    }
}

// ProscNoteModifyMessage
// n_set implementation with added external_id to allow modifying any note
// NOTE: Note-off doesn't need its own message; it is simply an n_set with gate=0
pub struct NoteModifyMessage {
    pub external_id_regex: String, // Modify all running external ids matching this regex
    pub delay_ms: u64,
    pub args: Vec<OscType>, // Args to set (same as in SNewTimedGateMessage)
}

impl NoteModifyMessage {
    pub fn new(message: &OscMessage) -> Result<NoteModifyMessage, String> {
        message.expect_addr("/note_modify")?;
        message.expect_args(2)?;

        let external_id_regex = message.get_string_at(0, "external id regex")?;
        let delay_ms = message.get_u64_at(1, "delay_ms")?;
        let args = message.get_varargs(2)?;

        Ok(NoteModifyMessage {
            external_id_regex,
            delay_ms,
            args,
        })
    }
}

// Example below of args in order with "" as category (= Empty)
// ["/play_sample", "my_unique_id", "example", 2, "", "arg1", 0.2, "arg2", 0.4, ...]
pub struct PlaySampleMessage {
    pub external_id: String,
    pub sample_pack: String,      // The parent dir of the sample file
    pub index: usize, // Sample number - either as plain order in dir or in a given category
    pub category: Option<String>, // TODO: Arbitrary string codes... is there a better way?
    pub delay_ms: u64,
    pub args: Vec<OscType>, // Args to set (same as in SNewTimedGateMessage)
}

impl PlaySampleMessage {
    pub fn new(message: &OscMessage) -> Result<PlaySampleMessage, String> {
        message.expect_addr("/play_sample")?;
        message.expect_args(5)?;

        let external_id = message.get_string_at(0, "external_id")?;
        let sample_pack = message.get_string_at(1, "sample_pack")?;
        let index = message.get_int_at(2, "index")?;
        let cat_arg = message.get_string_at(3, "category")?;
        let delay_ms = message.get_u64_at(4, "delay_ms")?;
        let args = message.get_varargs(5)?;

        if index < 0 {
            return Err("Index arg in sample message incompatible: negative".to_string());
        }

        let category = if cat_arg == "".to_string() {
            None
        } else {
            Some(cat_arg)
        };

        Ok(PlaySampleMessage {
            external_id,
            sample_pack,
            index: index as usize,
            category,
            delay_ms,
            args,
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
    pub end_beat: f32,
}

impl NRTRecordMessage {
    pub fn from_bundle(bundle: TaggedBundle) -> Result<NRTRecordMessage, String> {
        if &bundle.bundle_tag != "nrt_record" {
            return Err(format!(
                "Attempted to parse {} as nrt_record bundle",
                &bundle.bundle_tag
            ));
        }

        let info_msg = bundle.get_message(0)?;
        let message_bundle = bundle.get_bundle(1)?;

        info!(
            "Began parsing an nrt_record bundle with content size: {}",
            message_bundle.content.len()
        );

        let timed_messages: Vec<_> = message_bundle
            .content
            .iter()
            .map(|packet| {
                return match packet {
                    OscPacket::Bundle(bun) => {
                        let tagged = TaggedBundle::new(&bun)?;
                        info!("Parsing tagged bundle for NRT! {}", tagged.bundle_tag);
                        Ok(TimedOSCPacket::from_bundle(tagged)?)
                    }
                    _ => {
                        warn!("Unexpected non-bundle when unpacking timed messages bundle content");
                        Err("Unexpected non-bundle when unpacking timed messages bundle"
                            .to_string())
                    }
                };
            })
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
            end_beat,
        })
    }
}
