use std::{
    fs::File, io::Write, net::{SocketAddrV4, UdpSocket}, str::FromStr, sync::{Arc, Mutex}
};

use bigdecimal::BigDecimal;
use jdw_osc_lib::model::{OscArgHandler, TaggedBundle};
use log::{error, warn};
use rosc::{OscMessage, OscPacket, OscType};

use crate::{internal_osc_conversion::{self, SuperColliderNewMessage}, node_lookup::NodeIDRegistry, nrt_record::NRTConvert, osc_model::{LoadSampleMessage, NRTRecordMessage, NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, PlaySampleMessage}, sampling::{Sample, SamplePackDict}, sc_process_management::SCClient, scd_templating::{self, create_nrt_script}};

// Lots of code stolen from OSCStack to avoid having to work around client sharing over closures

const FUNNELED_TBUNDLES: [&str; 1] = ["batch-send"];

struct Interpreter {
    client: SCClient,
    reg: NodeIDRegistry,
    sample_pack_dict: SamplePackDict,
    synthef_snippets: Vec<String>,
    nrt_preloads: Vec<OscPacket>, // Packets to load on time 0.0 for all future nrt records,
    bpm: i32
}

impl Interpreter {
    fn interpret(&mut self, packet: OscPacket) {
        match packet {
            OscPacket::Message(osc_message) => {
                match osc_message.addr.as_str() {
                    "/free_notes" => {
                        let regex = osc_message.get_string_at(0, "Regex string").unwrap();

                        let node_ids = self.reg.regex_search_node_ids(&regex);

                        for node_id in node_ids {

                            let arg = OscType::Int(node_id);

                            self.client.send_with_delay(
                                OscPacket::Message(OscMessage {addr: "/n_free".to_string(), args: vec![arg]}),
                                0
                            );
                        }

                    },
                    "/set_bpm" => {
                        self.bpm = osc_message.get_int_at(0, "BPM value").unwrap();
                    },
                    "/note_on_timed" => {
                        let processed_message = NoteOnTimedMessage::new(&osc_message).unwrap();
                        let node_id = self.reg.create_node_id(&processed_message.external_id);
                        self.client.send_timed_packets(
                            processed_message.delay_ms,
                            processed_message.create_osc(node_id, self.bpm),
                        );
                    },
                    "/note_on" => {
                        let processed_message = NoteOnMessage::new(&osc_message).unwrap();
                        let node_id = self.reg.create_node_id(&processed_message.external_id);
                        self.client.send_timed_packets(
                            processed_message.delay_ms,
                            processed_message.create_osc(node_id),
                        );
                    },
                    "/play_sample" => {
                        let processed_message = PlaySampleMessage::new(&osc_message).unwrap();
                        let delay = processed_message.delay_ms;
                        let category = processed_message.category.clone().unwrap_or("".to_string());
                        let buffer_number = self.sample_pack_dict.find(
                            &processed_message.sample_pack,
                            processed_message.index,
                            &category,
                        ).map(|sample| sample.buffer_number).unwrap_or(0);
                        // TODO: Error handle missing sample
            
                        let internal_msg = processed_message.prepare(
                            buffer_number
                        );

                        let node_id = self.reg.create_node_id(&internal_msg.external_id);

                        // TODO: Adapt new osc conversion properly when everything is converted
                        self.client.send_timed_packets(
                            delay,
                            internal_msg.create_osc(node_id),
                        );
            
                    },
                    "/note_modify" => {
                        let processed_message = NoteModifyMessage::new(&osc_message).unwrap();

                        let node_ids = self.reg.regex_search_node_ids(&processed_message.external_id_regex);

                        self.client.send_timed_packets(
                            processed_message.delay_ms,
                            processed_message.create_osc(node_ids),
                        );
                    },
                    "/read_scd" => {
                        self.client.send_to_client(osc_message);
                    },
                    "/load_sample" => {
                        let resolved = LoadSampleMessage::new(&osc_message).unwrap();

                        // TODO: Probably used for NRT and perhaps possible to keep in some struct here instead 
                        let sample = self.sample_pack_dict.register_sample(resolved)
                            .unwrap();

                        self.client.send_to_client(OscMessage {
                            addr: "/read_scd".to_string(),
                            args: vec![
                                OscType::String(sample.get_buffer_load_scd()),
                            ],
                        });
                    },
                    "/clear_nrt" => {
                        self.nrt_preloads.clear();
                    },
                    "/create_synthdef" => {
                        // save scd in state, run scd in sclang
                        let definition = osc_message.get_string_at(0, "Synthdef scd string").unwrap();
                        
                        // TODO: Probably used for NRT and perhaps possible to keep in some struct here instead

                        if !self.synthef_snippets.contains(&definition) {
                            self.synthef_snippets.push(definition.clone());

                            let add_call = definition + ".add;";
                            self.client.send_to_client(OscMessage {
                                addr: "/read_scd".to_string(),
                                args: vec![OscType::String(add_call)],
                            });
                        }

                    },
                    _ => {},
                }
            }
            OscPacket::Bundle(osc_bundle) => {
                match TaggedBundle::new(&osc_bundle) {
                    Ok(tagged_bundle) => {
                        if FUNNELED_TBUNDLES.contains(&tagged_bundle.bundle_tag.as_str()) {
                            for packet in tagged_bundle.contents {
                                self.interpret(packet);
                            }
                        } else {
                            match tagged_bundle.bundle_tag.as_str() {
                                "nrt_preload" => {
                                    tagged_bundle.contents.iter().for_each(|packet| self.nrt_preloads.push(packet.clone()));
                                    println!("Preloaded nrt packets: {}", self.nrt_preloads.len());
                                },
                                "nrt_record" => {
                                    match NRTRecordMessage::from_bundle(tagged_bundle) {
                                        Ok(nrt_record_msg) => {
                        
                                            // Begin building the score rows with the sythdef creation strings
                                            let mut score_rows: Vec<String> = self.synthef_snippets.iter()
                                                .map(|def| def.clone() + ".asBytes")
                                                .map(|def| { return scd_templating::nrt_wrap_synthdef(&def); })
                                                .collect();
                        
                                            // Add the buffer reads for samples to the score
                                            for sample in self.sample_pack_dict.get_all_samples() {
                                                score_rows.push(sample.get_nrt_scd_row());
                                            }
                        
                        
                                            // Collect messages to be played as score rows along a timeline
                                            // TODO: Legacy internal osc conversion, but works for now and is a mess to clean up 
                                            let reg_handle = Arc::new(Mutex::new(NodeIDRegistry::new()));
                                            let dict_clone = self.sample_pack_dict.clone();
                                            let sample_pack_dict_arc = Arc::new(Mutex::new(dict_clone));
                                            let mut current_beat = BigDecimal::from_str("0.0").unwrap();

                                            let preload_rows: Vec<String> = self.nrt_preloads.iter()
                                                .flat_map(|packet| {
                        
                                                    let osc = internal_osc_conversion::resolve_msg(
                                                        packet.clone(),
                                                        sample_pack_dict_arc.clone()
                                                    ).map(|sc_msg| sc_msg.as_nrt_osc(
                                                        reg_handle.clone(), current_beat.clone()
                                                    )).unwrap_or(vec![]);
                        
                                                    osc
                                                })
                                                .map(|osc| osc.as_nrt_row())
                                                .collect();


                                            let timeline_score_rows: Vec<String> = nrt_record_msg.messages.iter()
                                                .flat_map(|timed_packet| {
                        
                                                    let osc = internal_osc_conversion::resolve_msg(
                                                        timed_packet.packet.clone(),
                                                        sample_pack_dict_arc.clone()
                                                    ).map(|sc_msg| sc_msg.as_nrt_osc(
                                                        reg_handle.clone(), current_beat.clone()
                                                    )).unwrap_or(vec![]);
                        
                                                    current_beat += timed_packet.time.clone();
                        
                                                    osc
                                                })
                                                .map(|osc| osc.as_nrt_row())
                                                .collect();
                                            
                                            let mut all_rows: Vec<String> = vec![];
                                            for row in preload_rows {
                                                all_rows.push(row);
                                            }
                                            for row in timeline_score_rows {
                                                all_rows.push(row);
                                            }
                                            
                        
                                            for m in all_rows {
                                                score_rows.push(m);
                                            }
                        
                                            let script = create_nrt_script(
                                                nrt_record_msg.bpm,
                                                &nrt_record_msg.file_name,
                                                nrt_record_msg.end_beat,
                                                score_rows,
                                            );

                                            // TODO: DEBUG STUFF 
                                            let mut file = File::create(&(nrt_record_msg.file_name.clone() + ".scd")).unwrap();
                                            file.write_all(script.as_bytes()).unwrap();
                                            println!("Saved NRT script as: {}", nrt_record_msg.file_name);
                        
                                            self.client.send_to_client(
                                                OscMessage {
                                                    addr: "/read_scd".to_string(),
                                                    args: vec![OscType::String(script)],
                                                }
                                            );
                        
                                            // TODO: Do something with the /nrt_done message
                        
                        
                                        }
                                        Err(e) => {
                                            error!("Failed to parse NRT record message: {}", e);
                                        }
                                }},
                                _ => {}
                            }
                        }
                    }
                    Err(msg) => warn!("Failed to parse bundle as tagged: {}", msg),
                };
            }
        }
    }
}

pub fn run(host_url: String, client: SCClient, sample_pack_dict: SamplePackDict, synthdef_snippets: Vec<String>) {
    let addr = match SocketAddrV4::from_str(&host_url) {
        Ok(addr) => addr,
        Err(e) => panic!("{}", e),
    };

    let sock = UdpSocket::bind(addr).unwrap();

    let mut buf = [0u8; 333072];

    let mut interpreter = Interpreter {
        client,
        reg: NodeIDRegistry::new(),
        sample_pack_dict: sample_pack_dict,
        synthef_snippets: synthdef_snippets,
        nrt_preloads: vec![],
        bpm: 120
    };

    loop {
        //let buf = [0u8; rosc::decoder::MTU];
        // TODO: Compare with size in struct declaration (should be same value)
        // THe MTU constant is way too low... I think.
        // Too low results in parts of large packets being dropped before receiving
        // Heck, might just be some kind of buffer thing where I'm supposed to read
        // multiple things but only end up reading the first.. .
        // UPDATE: Found no indication of this in documentation. :c

        match sock.recv_from(&mut buf) {
            Ok((size, _)) => {
                let (_rem, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();

                interpreter.interpret(packet);
            }
            Err(e) => {
                warn!("Failed to receive from socket {}", e);
            }
        };
    }
}
