use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use bigdecimal::BigDecimal;
use jdw_osc_lib::TimedOSCPacket;

use log::{debug, info, warn};
use regex::Regex;
use rosc::{OscMessage, OscPacket, OscType};

use crate::{NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, PlaySampleMessage, SampleDict};


/*
    Created notes often get assigned an external_id from the caller, which
        is then used to look up the actual nodeId used in the created internal
        supercollider osc message. IdRegistry keeps track of these variables.
 */
pub struct IdRegistry {
    pub registry: RefCell<HashMap<String, i32>>,
    curr_id: RefCell<i32>,
}

impl IdRegistry {
    pub fn new() -> IdRegistry {
        IdRegistry{registry: RefCell::new(HashMap::new()), curr_id: RefCell::new(100)}
    }

    // Assign and return a new unique node_id for the given external_id
    pub fn assign(&self, external_id: &str) -> i32 {
        let mut node_id = self.curr_id.clone().into_inner();
        node_id += 1;

        let mut new_reg = self.registry.clone().into_inner();
        new_reg.insert(external_id.to_string(), node_id);
        self.registry.replace(new_reg);

        self.curr_id.replace(node_id);

        node_id

    }

    // Get node_id by external_id, if present
    pub fn get(&self, external_id: String) -> Option<i32> {
        self.registry.borrow().get(&external_id).map(|int| int.clone())
    }

    // Get all node_ids matching regex
    pub fn get_regex(&self, external_id_regex: &str) -> Vec<i32> {

        let regex_attempt = Regex::new(external_id_regex);

        match regex_attempt {
            Ok(regex) => {

                let matching: Vec<_> = self.registry.borrow().iter()
                    .filter(|entry| regex.is_match(entry.0) )
                    .map(|entry| entry.1.clone())
                    .collect();

                debug!("Found {} running notes matching regex {}", matching.len(), external_id_regex);

                return matching;
            }
            Err(_) => {
                warn!("Provided regex {} is invalid", external_id_regex);
                vec![]
            }
        }

    }

    // Remove an external_id's node_id from the registry, if present
    pub fn clear(&self, external_id: String) {
        if self.registry.borrow().contains_key(&external_id) {
            let mut new_reg = self.registry.clone().into_inner();
            new_reg.remove(&external_id);
            self.registry.replace(new_reg);
        }
    }

}

pub trait InternalOSCMorpher {
    fn as_osc(&self, reg: Arc<Mutex<IdRegistry>>) -> Vec<TimedOSCPacket>;
    fn as_nrt_osc(&self, reg: Arc<Mutex<IdRegistry>>, start_time: BigDecimal) -> Vec<TimedOSCPacket> {
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

impl InternalOSCMorpher for NoteOnTimedMessage {
    fn as_osc(&self, reg: Arc<Mutex<IdRegistry>>) -> Vec<TimedOSCPacket> {

        let node_id = reg.lock().unwrap().assign(&self.external_id);
        let msg = create_s_new(node_id, &self.synth_name, &self.args);

        let message = OscMessage {
            addr: "/n_set".to_string(),
            args: vec![
                OscType::Int(node_id), // NodeID
                OscType::String("gate".to_string()), // gate=0 is note off
                OscType::Float(0.0)
            ]
        };

        let packet = OscPacket::Message(message.clone());
        let off_msg = TimedOSCPacket {time: self.gate_time.clone(), packet };

        vec![msg, off_msg]

    }

}

impl InternalOSCMorpher for NoteOnMessage {
    fn as_osc(&self, reg: Arc<Mutex<IdRegistry>>) -> Vec<TimedOSCPacket> {
        let node_id = reg.lock().unwrap().assign(&self.external_id);
        let msg = create_s_new(node_id, &self.synth_name, &self.args);

        vec![msg]
    }

}

impl InternalOSCMorpher for NoteModifyMessage {
    fn as_osc(&self, reg: Arc<Mutex<IdRegistry>>) -> Vec<TimedOSCPacket> {
        let node_ids = reg.lock().unwrap().get_regex(&self.external_id_regex);

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
pub struct PlaySampleInternalMessage {
    pub external_id: String, // TODO: Not currently part of original message - fix later for n_set compat
    pub args: Vec<OscType>
}


impl PlaySampleMessage {
    pub fn into_internal(self, samples: Arc<Mutex<SampleDict>>) -> PlaySampleInternalMessage {
        let mut base_args = self.args.clone();

        let buf_nr = samples
            .lock()
            .unwrap()
            .get_buffer_number(&self.sample_pack, self.index, self.category.clone())
            .unwrap_or(0); // Should probably be some kind of error, but for now default to base buf


        if base_args.iter()
            .map(|arg| arg.clone())
            .find(|arg| arg.clone().string().is_some_and(|a| a == "buf"))
            .is_some() {
            warn!("Sample play request contained a preset arg for 'buf' which can impact sample playback.");
        }

        base_args.push(OscType::String("buf".to_string()));
        base_args.push(OscType::Int(buf_nr));

        PlaySampleInternalMessage {
            external_id: self.external_id,
            args: base_args
        }
    }
}

impl InternalOSCMorpher for PlaySampleInternalMessage {
    fn as_osc(&self, reg: Arc<Mutex<IdRegistry>>) -> Vec<TimedOSCPacket> {
        let node_id = reg.lock().unwrap().assign(&self.external_id);
        vec![create_s_new(
            node_id,
            "sampler", // Refers to sampler.scd, the "synth" used to play buffer samples
            &self.args
        )]
    }

}