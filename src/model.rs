use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rosc::OscType;

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
pub struct ProscNoteCreateMessage {
    pub target: String,
    pub external_id: String,
    pub args: HashMap<String, f32>,
}

impl ProscNoteCreateMessage {
    pub fn get_arg_vec(&self) -> Vec<OscType> {
        let mut vec: Vec<OscType> = Vec::new();
        for (k,v) in self.args.iter() {
            vec.push(OscType::String(k.to_string()));
            vec.push(OscType::Float(v.clone()));
        }
        vec
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
        let mut vec: Vec<OscType> = Vec::new();
        for (k,v) in self.args.iter() {
            vec.push(OscType::String(k.to_string()));
            vec.push(OscType::Float(v.clone()));
        }
        vec
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