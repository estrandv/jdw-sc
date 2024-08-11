use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use bigdecimal::{BigDecimal, FromPrimitive};
use jdw_osc_lib::model::TimedOSCPacket;
use log::{info, warn};
use rosc::{OscMessage, OscPacket, OscType};
use crate::node_lookup::NodeIDRegistry;
use crate::osc_model::{NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, PlaySampleMessage};
use crate::sampling::SamplePackDict;

pub trait SuperColliderNewMessage {
    fn create_osc(&self, node_id: i32) -> Vec<TimedOSCPacket>;
    fn create_nrt_osc(&self, node_id: i32, start_time: BigDecimal) -> Vec<TimedOSCPacket>;
}

// TODO: Legacy, moving away from ARC reliance
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


// TODO: Util lib 

fn seconds_from_beats(bpm: i32, beats: BigDecimal) -> BigDecimal {
    let beats_per_second = BigDecimal::from_i32(bpm).unwrap() / BigDecimal::from_i64(60).unwrap();
    return beats / beats_per_second; 
}

impl NoteOnTimedMessage {
    pub fn create_osc(&self, node_id: i32, bpm: i32) -> Vec<TimedOSCPacket> {
        let on_message = create_s_new(node_id, &self.synth_name, &self.args);

        let off_packet = OscPacket::Message(OscMessage {
            addr: "/n_set".to_string(),
            args: vec![
                OscType::Int(node_id), // NodeID
                OscType::String("gate".to_string()), // gate=0 is note off
                OscType::Float(0.0)
            ]
        });

        // Calculate time of off-message as seconds-from-beats

        let seconds = seconds_from_beats(bpm, self.gate_time.clone());
        //info!("Sustain time was {}sec", seconds.clone());

        let off_message = TimedOSCPacket {time: seconds, packet: off_packet };

        vec![on_message, off_message]
    }

}

impl NoteOnMessage {
    pub fn create_osc(&self, node_id: i32) -> Vec<TimedOSCPacket> {
        let msg = create_s_new(node_id, &self.synth_name, &self.args);

        vec![msg]
    }

}

impl SuperColliderMessage for NoteOnMessage {
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        let node_id = reg.lock().unwrap().create_node_id(&self.external_id);
        let msg = create_s_new(node_id, &self.synth_name, &self.args);

        vec![msg]
    }

}

impl NoteModifyMessage {
    pub fn create_osc(&self, node_ids: Vec<i32>) -> Vec<TimedOSCPacket> {
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

}

impl SuperColliderMessage for PreparedPlaySampleMessage {
    
    fn as_osc(&self, reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        let node_id = reg.lock().unwrap().create_node_id(&self.external_id);
        vec![create_s_new(
            node_id,
            "sampler", // The "synth" used to play buffer samples
            &self.args
        )]
    }

}

impl PreparedPlaySampleMessage {
    pub fn create_osc(&self, node_id: i32) -> Vec<TimedOSCPacket> {
        vec![create_s_new(
            node_id,
            "sampler", // The "synth" used to play buffer samples
            &self.args
        )]
    }

}


// TODO: Not happy with dict usage, but at least this moves it out of the way for now...
pub fn resolve_msg(packet: OscPacket, dict: Arc<Mutex<SamplePackDict>>) -> Option<Box<dyn SuperColliderMessage>> {

    let msg = match packet {
        OscPacket::Message(msg) => {
            Some(msg)
        }
        OscPacket::Bundle(_) => {
            warn!("UNEXPECTED BUNDLE IN RESOLVE_MSG");
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
        msgtype => {
            warn!("Unknown message type: {}", msgtype);
            None
        }

    };

    return sc_msg;
}