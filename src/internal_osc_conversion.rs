use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use bigdecimal::BigDecimal;
use jdw_osc_lib::model::TimedOSCPacket;
use log::{debug, error, info, warn};
use regex::Regex;
use rosc::{OscMessage, OscPacket, OscType};

use crate::{NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, PlaySampleMessage, SamplePackCollection};
use crate::node_lookup::NodeIDRegistry;
use crate::sampling::SamplePackDict;

pub trait SuperColliderMessage {
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket>;
    fn as_nrt_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>, start_time: BigDecimal) -> Vec<TimedOSCPacket> {
        self.as_osc(reg).iter()
            .map(|msg| TimedOSCPacket {
                time: msg.time.clone() + start_time.clone(),
                packet: msg.packet.clone()
            }).collect()
    }
}

fn create_s_new(
    node_id: i32,
    synth_name: &str,
    msg_args: &Vec<OscType>
) -> TimedOSCPacket {

    let mut final_args = vec![
        OscType::String(synth_name.to_string()),
        OscType::Int(node_id), // NodeID
        OscType::Int(0), // Group?
        OscType::Int(0), // Group placement?
    ];

    final_args.extend(msg_args.clone());

    let message = OscMessage {
        addr: "/s_new".to_string(),
        args:  final_args
    };

    let packet = OscPacket::Message(message.clone());

    TimedOSCPacket {time: BigDecimal::from_str("0.0").unwrap(), packet}
}

impl SuperColliderMessage for NoteOnTimedMessage {
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {

        let node_id = reg.lock().unwrap().create_node_id(&self.external_id);
        let on_message = create_s_new(node_id, &self.synth_name, &self.args);

        let off_packet = OscPacket::Message(OscMessage {
            addr: "/n_set".to_string(),
            args: vec![
                OscType::Int(node_id), // NodeID
                OscType::String("gate".to_string()), // gate=0 is note off
                OscType::Float(0.0)
            ]
        });

        let off_message = TimedOSCPacket {time: self.gate_time.clone(), packet: off_packet };

        vec![on_message, off_message]

    }

}

impl SuperColliderMessage for NoteOnMessage {
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        let node_id = reg.lock().unwrap().create_node_id(&self.external_id);
        let msg = create_s_new(node_id, &self.synth_name, &self.args);

        vec![msg]
    }

}

impl SuperColliderMessage for NoteModifyMessage {
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        let node_ids = reg.lock().unwrap().regex_search_node_ids(&self.external_id_regex);

        node_ids.iter()
            .map(|id| {
                let mut final_ars = vec![
                    OscType::Int(id.clone())
                ];

                final_ars.extend(self.args.clone());

                let message = OscMessage {
                    addr: "/n_set".to_string(),
                    args: final_ars
                };

                let packet = OscPacket::Message(message.clone());

                TimedOSCPacket {time: BigDecimal::from_str("0.0").unwrap(), packet }
            }).collect()

    }

}


// Transitional struct used to keep sample lookup logic out of osc_model
// external osc message -> PlaySampleMessage -> PlaySampleInternalMessage -> internal osc, etc.
pub struct PreparedPlaySampleMessage {
    pub external_id: String, // TODO: Not currently part of original message - fix later for n_set compat
    pub args: Vec<OscType>
}


impl PlaySampleMessage {

    pub fn prepare(self, buffer_number: i32) -> PreparedPlaySampleMessage {
        let mut base_args = self.args.clone();

        if base_args.iter()
            .map(|arg| arg.clone())
            .find(|arg| arg.clone().string().is_some_and(|a| a == "buf"))
            .is_some() {
            warn!("Sample play request contained a preset arg for 'buf', which can impact sample playback.");
        }

        base_args.push(OscType::String("buf".to_string()));
        base_args.push(OscType::Int(buffer_number));

        PreparedPlaySampleMessage {
            external_id: self.external_id,
            args: base_args
        }
    }

    // TODO: Legacy
    pub fn with_buffer_arg(self, samples: Arc<Mutex<SamplePackCollection>>) -> PreparedPlaySampleMessage {
        let mut base_args = self.args.clone();

        let buf_nr = samples
            .lock()
            .unwrap()
            .category_to_buf(&self.sample_pack, self.index, self.category.clone())
            .unwrap_or(0); // Should probably be some kind of error, but for now default to base buf

        if base_args.iter()
            .map(|arg| arg.clone())
            .find(|arg| arg.clone().string().is_some_and(|a| a == "buf"))
            .is_some() {
            warn!("Sample play request contained a preset arg for 'buf', which can impact sample playback.");
        }

        base_args.push(OscType::String("buf".to_string()));
        base_args.push(OscType::Int(buf_nr));

        PreparedPlaySampleMessage {
            external_id: self.external_id,
            args: base_args
        }
    }

}

impl SuperColliderMessage for PreparedPlaySampleMessage {
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        let node_id = reg.lock().unwrap().create_node_id(&self.external_id);
        vec![create_s_new(
            node_id,
            "sampler", // Refers to sampler.scd, the "synth" used to play buffer samples
            &self.args
        )]
    }

}


// TODO: Not happy with dict usage, but at least this moves it out of the way for now...
pub fn resolve_msg(packet: OscPacket, dict: Arc<Mutex<SamplePackDict>>) -> Box<dyn SuperColliderMessage> {

    let msg = match packet {
        OscPacket::Message(msg) => {
            Some(msg)
        }
        OscPacket::Bundle(_) => {
            None
        }
    }.unwrap();

    let sc_msg: Option<Box<dyn SuperColliderMessage>> = match msg.addr.as_str() {
        "/note_on_timed" => {
            Some(Box::new(NoteOnTimedMessage::new(&msg.clone())
                .unwrap()))
        }
        "/play_sample" => {
            Some(Box::new(PlaySampleMessage::new(&msg.clone()).map(|play_sample| {
                let cat = play_sample.category.clone().unwrap_or("".to_string());
                let buf = dict.lock().unwrap().find(
                    &play_sample.sample_pack.to_string(),
                    play_sample.index,
                    &cat
                ).map(|sample| sample.buffer_number).unwrap_or(0);
                // TODO: warn on missing sample
                play_sample.prepare(buf)
            }).unwrap()))
        }
        "/note_on" => {
            Some(Box::new(NoteOnMessage::new(&msg.clone())
                .unwrap()))
        }
        "/note_modify" => {
            Some(Box::new(NoteModifyMessage::new(&msg.clone())
                .unwrap()))
        }
        _ => {
            None
        }

    };

    return sc_msg.unwrap();
}