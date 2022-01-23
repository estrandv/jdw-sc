use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rosc::OscType;
use crate::samples::SampleDict;
use std::sync::{Mutex, Arc};

fn map_args(args: &HashMap<String, f32>) -> Vec<OscType> {
    let mut vec: Vec<OscType> = Vec::new();
    for (k,v) in args.iter() {
        vec.push(OscType::String(k.to_string()));
        vec.push(OscType::Float(v.clone()));
    }
    vec
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningNote {
    pub synth: String,
    pub external_id: String,
    pub tone: i32
}

pub trait ZeroMQSendable {
    fn export(&self) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdwSequencerBatchMsg {
    pub json: Vec<String>
}

/*
    Note that there is basically no useful difference between this and note on atm.
    Considerations need to be made: should missing "sus" arg be the only divider
        between note_on and autogate, removing the need for separate messages?
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdwPlayNoteMsg {
    pub target: String,
    pub source: String,
    // pub gate_time: f32, // In secs?
    pub args: HashMap<String, f32>,
}

impl JdwPlayNoteMsg {
    pub fn get_arg_vec(&self) -> Vec<OscType> {
        map_args(&self.args)
    }

    pub fn get_gate_time(&self) -> f32 {
        match self.args.get("sus") {
            Some(time) => *time,
            None => 1.0 // Probably not a sane default; I guess we should really return optional
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdwPlaySampleMsg {
    pub family: String, // e.g. "hh", uses buffer data lookup to find a named sample
    pub target: String, // Not the same behaviour as for synths; this is the sample pack name
    pub index: i32, // Similar to args["freq"] but instead the number of the sample in the pack (or family)
    pub args: HashMap<String, f32>,
}

impl JdwPlaySampleMsg {
    pub fn get_arg_vec(&self, samples: Arc<Mutex<SampleDict>>) -> Vec<OscType> {

        let mut base_args = map_args(&self.args);

        let category = match self.family != "".to_string() {
            true => Option::None,
            false => Option::Some(self.family.to_string())
        };

        let buf_nr = samples.lock().unwrap().get_buffer_number(&self.target, self.index, category)
            .unwrap_or(0); // Should probably be some kind of error, but for now default to base buf

        base_args.push(OscType::String("buf".to_string()));
        base_args.push(OscType::Int(buf_nr));

        base_args

    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProscNoteCreateMessage {
    pub target: String,
    pub external_id: String,
    pub args: HashMap<String, f32>,
}

impl ProscNoteCreateMessage {
    pub fn get_arg_vec(&self) -> Vec<OscType> {
        map_args(&self.args)
    }

    pub fn get_gate_time(&self) -> Option<f32> {
        match self.args.get("sus") {
            Some(time) => Option::Some(*time),
            None => None
        }
    }
}

impl ZeroMQSendable for ProscNoteCreateMessage {
    fn export(&self) -> String {
        return format!("JDW.ADD.NOTE::{}", serde_json::to_string(self).unwrap());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProscNoteModifyMessage {
    pub external_id: String,
    pub args: HashMap<String, f32>,
}

impl ProscNoteModifyMessage {
    pub fn get_arg_vec(&self) -> Vec<OscType> {
        map_args(&self.args)
    }
}


impl ZeroMQSendable for ProscNoteModifyMessage {
    fn export(&self) -> String {
        return format!("JDW.NSET.NOTE::{}", serde_json::to_string(self).unwrap());
    }
}

struct NoteOnMessage {
    target: String,
    tone: i32,
    velocity: i32
}

struct NoteOffMessage {
    target: String,
    tone: i32,
    velocity: i32
}

struct PitchBendMessage {
    target: String,
    lsb: i32,
    msb: i32
}

struct AfterTouchMessage {
    target: String,
    tone: i32,
    touch: i32
}

struct ControllerMessage {
    attribute: String,
    controller: i32,
    value: i32
}