use std::convert::TryInto;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use bigdecimal::BigDecimal;
use jdw_osc_lib::model::TimedOSCPacket;
use log::{debug, error, info, warn};
use rosc::{OscBundle, OscMessage, OscPacket, OscType};
use crate::internal_osc_conversion::SuperColliderMessage;
use crate::scd_templating;
use crate::SamplePackCollection;
use crate::PlaySampleMessage;
use crate::NRTRecordMessage;
use crate::NoteOnTimedMessage;
use crate::NoteOnMessage;
use crate::NoteModifyMessage;
use crate::create_nrt_script;
use crate::node_lookup::NodeIDRegistry;
use crate::samples::Sample;
use crate::sampling::SamplePackDict;

impl Sample {
    // Buffer load as-osc, suitable for loading into the NRT server
    pub fn to_nrt_scd_row(&self, dir: &str) -> String {
        // TODO: TEmplate-friendly pieces
        // NOTE: If you want relative paths in scd you can do: File.getcwd +/+
        let ret = format!(
            "[0.0, (Buffer.new(server, 44100 * 8.0, 2, bufnum: {})).allocReadMsg(\"{}\")]",
            self.buffer_nr,
            dir.to_string() + "/" + &self.file_name.to_string(),
        );

        ret
    }
}


struct NRTPacketConverter {
    reg_handle: Arc<Mutex<NodeIDRegistry>>,
    buffer_handle: Arc<Mutex<SamplePackCollection>>,
    current_beat: BigDecimal,
}

impl NRTPacketConverter {
    fn process_packet(&self, timed_packet: &TimedOSCPacket) -> Vec<TimedOSCPacket> {
        return match &timed_packet.packet {
            OscPacket::Message(msg) => {
                let beat = self.current_beat.clone();
                let registry = self.reg_handle.clone();

                match msg.addr.as_str() {
                    "/note_on_timed" => {
                        NoteOnTimedMessage::new(&msg.clone())
                            .unwrap()
                            .as_nrt_osc(registry, beat)
                    }
                    "/play_sample" => {
                        PlaySampleMessage::new(msg)
                            .unwrap()
                            .with_buffer_arg(
                                self.buffer_handle.clone()
                            )
                            .as_nrt_osc(registry, beat)
                    }
                    "/note_on" => {
                        NoteOnMessage::new(msg)
                            .unwrap()
                            .as_nrt_osc(registry, beat)
                    }
                    "/note_modify" => {
                        NoteModifyMessage::new(msg)
                            .unwrap()
                            .as_nrt_osc(registry, beat)
                    }
                    other => {
                        error!("UNKNOWN NRT MSG {}", other);
                        vec![] // TODO: Wrap in some default handler - important part is using current_time
                    }
                }
            }
            OscPacket::Bundle(bun) => {
                warn!("NRT support for timed bundles not yet implemented");
                vec![]
            }
        };
    }

    fn process_packets(&mut self, packets: &Vec<TimedOSCPacket>) -> Vec<TimedOSCPacket> {
        let mut result_vector: Vec<TimedOSCPacket> = Vec::new();

        for msg in packets {

            // Each contained message must first be converted to its internal equivalent
            let rows = self.process_packet(msg);

            self.current_beat += msg.time.clone();
            result_vector.extend(rows);
        }

        // Ensure all messages are in order
        result_vector.sort_by(|a, b| a.time.cmp(&b.time));

        result_vector
    }
}

impl NRTRecordMessage {
// TODO: Handle all the unwraps

    pub fn get_processed_messages(
        &self,
        buffer_handle: Arc<Mutex<SamplePackCollection>>,
    ) -> Vec<TimedOSCPacket> {
        let registry = NodeIDRegistry::new();
        let reg_handle = Arc::new(Mutex::new(registry));

        let mut processor = NRTPacketConverter {
            reg_handle,
            buffer_handle,
            current_beat: BigDecimal::from_str("0.0").unwrap(),
        };

        info!("Processing messages with length: {}", self.messages.len());

        processor.process_packets(&self.messages)
    }
}

pub trait NRTConvert {
    fn as_nrt_row(&self) -> String;
}

impl NRTConvert for TimedOSCPacket {
    // Sort of a debug format; display as a string of values: [/s_new, "arg", 2.0, etc.]
    fn as_nrt_row(&self) -> String {
        let mut row_template = "[ {:time}, [\"{:adr}\",{:args}] ]".to_string();

        // TODO: Gonna cheat here for now. We're supposed to do the whole processor routine...
        let msg = match &self.packet {
            OscPacket::Message(msg) => { Some(msg.clone()) }
            OscPacket::Bundle(_) => { None }
        };

        let args: Vec<_> = msg.clone().unwrap().args.iter()
            .map(|arg| {
                let ball = match arg {
                    OscType::Int(val) => {
                        format!("{}", val)
                    }
                    OscType::Float(val) => {
                        format!("{:.5}", val)
                    }
                    OscType::String(val) => {
                        format!("\"{}\"", val)
                    }
                    _ => {
                        // TODO: Implement everything some day
                        "err".to_string()
                    }
                };
                ball
            }).collect();

        row_template = row_template.replace("{:time}", &format!("{:.5}", &self.time));
        row_template = row_template.replace("{:adr}", &format!("{}", msg.unwrap().addr));

        let arg_string = args.join(",");

        row_template = row_template.replace("{:args}", &arg_string);

        row_template
    }
}


// TODO: Legacy
pub fn get_nrt_record_scd(
    msg: &NRTRecordMessage,
    buffer_handle: Arc<Mutex<SamplePackCollection>>,
    synthdef_scds: Arc<Mutex<Vec<String>>>,
) -> Result<String, String> {
    let rows = msg.get_processed_messages(
        buffer_handle.clone()
    );

    let row_chunk: Vec<_> = rows.iter()
        .map(|m| m.as_nrt_row()).collect();

    let buffer_load_row_chunk = buffer_handle
        .lock()
        .unwrap()
        .as_nrt_buffer_load_rows();

    let mut synthdefs: Vec<String> = synthdef_scds.lock().unwrap().iter().map(|def| def.clone() + ".asBytes").collect();
    // TODO: Compat reading of the scd dir - should eventually be removed in favour of "synthdef_scds".
    let filedefs = scd_templating::read_all_synths("asBytes");
    for f in filedefs {
        synthdefs.push(f.clone());
    }

    let synth_rows: Vec<_> = synthdefs.iter()
        .map(|def| { return scd_templating::nrt_wrap_synthdef(def); })
        .collect();

    let mut all_nrt_rows: Vec<String> = vec![];
    all_nrt_rows.extend(buffer_load_row_chunk);
    all_nrt_rows.extend(synth_rows);

    all_nrt_rows.extend(row_chunk);

    create_nrt_script(
        msg.bpm,
        &msg.file_name,
        msg.end_beat,
        all_nrt_rows,
    )
}

