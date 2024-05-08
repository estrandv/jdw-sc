#![feature(result_flattening)]


use std::process::exit;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use bigdecimal::BigDecimal;
use home::home_dir;
use jdw_osc_lib::model::{OscArgHandler, TimedOSCPacket};
use jdw_osc_lib::osc_stack::OSCStack;
use log::{error, info};
use rosc::{OscMessage, OscType};
use simple_logger::SimpleLogger;
use crate::config::APPLICATION_IN_PORT;
use crate::internal_osc_conversion::SuperColliderMessage;
use crate::node_lookup::NodeIDRegistry;
use crate::nrt_record::NRTConvert;
use crate::osc_model::{LoadSampleMessage, NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, NRTRecordMessage, PlaySampleMessage};
use crate::sampling::SamplePackDict;
use crate::sc_process_management::SCProcessManager;
use crate::scd_templating::create_nrt_script;

mod sc_process_management;
mod scd_templating;
mod osc_model;
mod nrt_record;
mod config;
mod internal_osc_conversion;
mod node_lookup;
mod sampling;


/*
    TODOs:
    
    - SET_BPM
        - Should store a BPM for NRT as well as set it for the running server if that is possible (in order to auto-adjust args)
        - Since this will be saved in state, it becomes less important that NRT_RECORD has a bpm parameter 
        - Should also adjust GATE_TIME, since the current logic uses a time-stamp for the gate-off message
*/

fn main() {

    // Handles all log macros, e.g. "warn!()" to print info in terminal
    SimpleLogger::new()
        .with_level(config::LOG_LEVEL)
        .init().unwrap();

    /*
        Prepare thread handler for the main supercollider instance
     */
    let sc_process_manager = SCProcessManager::new().unwrap_or_else(|err| {
        error!("ERROR BOOTING SUPERCOLLIDER: {:?}", err);
        exit(0)
    });
    let sc_arc = Arc::new(Mutex::new(sc_process_manager));
    let sc_arc_in_ctrlc = sc_arc.clone();

    // Terminate supercollider on ctrl+c
    ctrlc::set_handler(move || {
        info!("Thread abort requested");
        sc_arc_in_ctrlc.lock().unwrap().terminate();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    /*
        Wait for the custom /init message from the server (see start_server.scd.template).
     */
    match sc_arc.lock().unwrap().await_response(
        "/init",
        vec![OscType::String("ok".to_string())],
        Duration::from_secs(10),
    ) {
        Err(e) => {
            error!("{}", e);
            sc_arc.lock().unwrap().terminate();
        }
        Ok(()) => ()
    };

    info!("Server online!");

    // TODO: Sampler synth must be created on startup - could potentially be part of boot script!

    let sample_pack_dict = SamplePackDict::new();
    let sample_pack_dict_arc = Arc::new(Mutex::new(sample_pack_dict));

    let mut sample_pack_dir = home_dir().unwrap();
    sample_pack_dir.push("sample_packs");

    // Populated via osc messages, used e.g. for NRT recording
    let loaded_synthdef_snippets: Vec<String> = Vec::new();
    let synthdef_snippets_arc = Arc::new(Mutex::new(loaded_synthdef_snippets));

    let node_reg = Arc::new(Mutex::new(NodeIDRegistry::new()));


    // Ready the sampler synth - similar to a create_synthdef call.
    let sampler_def = scd_templating::read_scd_file("sampler.scd");
    synthdef_snippets_arc.lock().unwrap().push(sampler_def.clone());
    sc_arc.lock().unwrap().send_to_client(OscMessage {
        addr: "/read_scd".to_string(),
        args: vec![
            OscType::String(sampler_def + ".add;"),
        ],
    });

    fn beep(freq: f32, node_reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        NoteOnTimedMessage::new(&OscMessage {
            addr: "/note_on_timed".to_string(),
            args: vec![
                OscType::String("default".to_string()),
                OscType::String("launch_ping".to_string()),
                OscType::String("0.125".to_string()),
                OscType::Int(0),
                OscType::String("freq".to_string()),
                OscType::Float(freq),
                OscType::String("amp".to_string()),
                OscType::Float(1.0),
            ],
        }).unwrap().as_osc(node_reg)
    }

    // Play a welcoming tune in a really obtuse way.
    for i in [130.81, 146.83, 196.00] {
        SCProcessManager::send_timed_packets(0, sc_arc.clone(), beep(i, node_reg.clone()));
        sleep(Duration::from_millis(125));
    }

    let reg = Arc::new(Mutex::new(NodeIDRegistry::new()));

    info!("Startup completed, polling for messages ...");

    OSCStack::init(config::get_addr(APPLICATION_IN_PORT))
        .on_message("/note_on_timed", &|msg| {
            let processed_message = NoteOnTimedMessage::new(&msg).unwrap();
            SCProcessManager::send_timed_packets(
                processed_message.delay_ms,
                sc_arc.clone(),
                processed_message.as_osc(reg.clone()),
            );
        })
        .on_message("/note_on", &|msg| {
            let processed_message = NoteOnMessage::new(&msg).unwrap();
            SCProcessManager::send_timed_packets(
                processed_message.delay_ms,
                sc_arc.clone(),
                processed_message.as_osc(reg.clone()),
            );
        })
        .on_message("/play_sample", &|msg| {

            let processed_message = PlaySampleMessage::new(&msg).unwrap();
            let delay = processed_message.delay_ms;
            let category = processed_message.category.clone().unwrap_or("".to_string());
            let buffer_number = sample_pack_dict_arc.lock().unwrap().find(
                &processed_message.sample_pack,
                processed_message.index,
                &category,
            ).map(|sample| sample.buffer_number).unwrap_or(0);
            // TODO: Error handle missing sample

            let internal_msg = processed_message.prepare(
                buffer_number
            );
            SCProcessManager::send_timed_packets(
                delay,
                sc_arc.clone(),
                internal_msg.as_osc(reg.clone()),
            );
        })
        .on_message("/note_modify", &|msg| {
            let processed_message = NoteModifyMessage::new(&msg).unwrap();
            SCProcessManager::send_timed_packets(
                processed_message.delay_ms,
                sc_arc.clone(),
                processed_message.as_osc(reg.clone()),
            );
        })
        .on_message("/read_scd", &|msg| {
            sc_arc.lock().unwrap().send_to_client(msg);
        })
        .on_message("/load_sample", &|msg| {
            let resolved = LoadSampleMessage::new(&msg).unwrap();
            let sample = sample_pack_dict_arc.lock().unwrap().register_sample(resolved)
                .unwrap();
            sc_arc.lock().unwrap().send_to_client(OscMessage {
                addr: "/read_scd".to_string(),
                args: vec![
                    OscType::String(sample.get_buffer_load_scd()),
                ],
            });
        })
        .on_message("/create_synthdef", &|msg| {

            // save scd in state, run scd in sclang
            let definition = msg.get_string_at(0, "Synthdef scd string").unwrap();
            synthdef_snippets_arc.lock().unwrap().push(definition.clone());
            let add_call = definition + ".add;";
            sc_arc.lock().unwrap().send_to_client(OscMessage {
                addr: "/read_scd".to_string(),
                args: vec![
                    OscType::String(add_call),
                ],
            });
        })
        .on_tbundle("nrt_record", &|tagged_bundle| {

            match NRTRecordMessage::from_bundle(tagged_bundle) {
                Ok(nrt_record_msg) => {

                    // Begin building the score rows with the sythdef creation strings
                    let mut score_rows: Vec<String> = synthdef_snippets_arc.lock().unwrap().iter()
                        .map(|def| def.clone() + ".asBytes")
                        .map(|def| { return scd_templating::nrt_wrap_synthdef(&def); })
                        .collect();

                    // Add the buffer reads for samples to the score
                    for sample in sample_pack_dict_arc.lock().unwrap().get_all_samples() {
                        score_rows.push(sample.get_nrt_scd_row());
                    }


                    // Collect messages to be played as score rows along a timeline
                    let reg_handle = Arc::new(Mutex::new(NodeIDRegistry::new()));
                    let mut current_beat = BigDecimal::from_str("0.0").unwrap();
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

                    for m in timeline_score_rows {
                        score_rows.push(m);
                    }

                    let script = create_nrt_script(
                        nrt_record_msg.bpm,
                        &nrt_record_msg.file_name,
                        nrt_record_msg.end_beat,
                        score_rows,
                    );

                    sc_arc.lock().unwrap().send_to_client(
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
            }

        })
        // Treat each packet in batch-send as a separately interpreted packet
        .funnel_tbundle("batch-send")
        .begin();
}
