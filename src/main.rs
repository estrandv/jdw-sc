#![feature(result_flattening)]

use std::cell::RefCell;
use std::{f32, f64, ops};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use home::home_dir;
use jdw_osc_lib::model::{TaggedBundle, TimedOSCPacket};
use jdw_osc_lib::osc_stack::OSCStack;
use log::{debug, error, info, LevelFilter, warn};
use rosc::{OscMessage, OscPacket, OscType};
use simple_logger::SimpleLogger;
use subprocess::{Exec, Popen, PopenConfig, Redirection};

use crate::config::APPLICATION_IN_PORT;
use crate::internal_osc_conversion::SuperColliderMessage;
use crate::node_lookup::NodeIDRegistry;
use crate::osc_model::{NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, NRTRecordMessage, PlaySampleMessage};
use crate::samples::SamplePackCollection;
use crate::scd_templating::create_nrt_script;
use crate::supercollider::SCProcessManager;

mod supercollider;
mod scd_templating;
mod samples;
mod osc_model;
mod nrt_record;
mod config;
mod internal_osc_conversion;
mod node_lookup;
mod util;
mod sample_sorting;

/*
    TODO: General refactoring of the whole main loop. 
    The Supercollider class is currently a hodge podge of different paradigms since it has evolved organically 
        from little to medium rust knowledge on my part. 
    Ideally, we want to separate state and logic better, like we have in jdw-sequencer. 
    A poller should process incoming messages into neat vectors/buffers of "messages to send to sclang or scsynth"
        and then use a minimal and transparent amount of locks to accomplish that. 
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
        },
        Ok(()) => ()
    };

    info!("Server online!");

    /*
        Use the synth definitions from the synths dir to ready custom scd messages.
        The messages then create these synthdefs on the server using the sclang client.
     */
    let synth_defs = scd_templating::read_all_synths("add;");

    // See start_server.scd.template for the /read_scd definition
    for synth_def in synth_defs {
        sc_arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args: vec![OscType::String(synth_def)],
            }
        )
    }

    /*
        Prepare sample players. All samples are read into buffers via read_scd on the sclang client.
        The sample dict struct keeps track of which buffer index belongs to which sample pack.
     */

    let mut sample_pack_dir = home_dir().unwrap();
    sample_pack_dir.push("sample_packs");

    let sample_dict = SamplePackCollection::create(&sample_pack_dir).unwrap_or_else(|e| {
        error!("Unable to read buffer data: {} - no samples will be provided", e);
        SamplePackCollection::empty()
    });

    let buffer_string = sample_dict.as_buffer_load_scd();

    let sample_dict_arc = Arc::new(Mutex::new(sample_dict));

    if !buffer_string.is_empty() {
        sc_arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args: vec![OscType::String(buffer_string)],
            }
        );

        // Message is added to the end of the buffer load scd to signify a completed load call.
        match sc_arc.lock().unwrap()
            .await_response(
                "/buffers_loaded",
                vec![OscType::String("ok".to_string())],
                Duration::from_secs(10),
            ) {
            Err(e) => {
                error!("{}", e);
                sc_arc.lock().unwrap().terminate();
            },
            Ok(()) => ()
        };
    }

    ///////////////////////////


    let node_reg = Arc::new(Mutex::new(NodeIDRegistry::new()));

    fn beep(freq: f32, node_reg: Arc<Mutex<NodeIDRegistry>>) -> Vec<TimedOSCPacket> {
        NoteOnTimedMessage::new(&OscMessage {
            addr: "/note_on_timed".to_string(),
            args: vec![
                OscType::String("default".to_string()),
                OscType::String("launch_ping".to_string()),
                OscType::String("0.125".to_string()),
                OscType::String("freq".to_string()),
                OscType::Float(freq),
                OscType::String("amp".to_string()),
                OscType::Float(1.0),

            ],
        }).unwrap().as_osc(node_reg)
    }

    // Play a welcoming tune in a really obtuse way.
    for i in [130.81, 146.83, 196.00] {
        SCProcessManager::send_timed_packets(sc_arc.clone(), beep(i, node_reg.clone()));
        sleep(Duration::from_millis(125));
    }

    let reg = Arc::new(Mutex::new(NodeIDRegistry::new()));

    info!("Startup completed, polling for messages ...");

    OSCStack::init(config::get_addr(APPLICATION_IN_PORT))
        .on_message("/note_on_timed", &|msg| {
            let processed_message = NoteOnTimedMessage::new(&msg).unwrap();
            SCProcessManager::send_timed_packets(
                sc_arc.clone(),
                processed_message.as_osc(reg.clone()),
            );
        })
        .on_message("/note_on", &|msg| {
            let processed_message = NoteOnMessage::new(&msg).unwrap();
            SCProcessManager::send_timed_packets(
                sc_arc.clone(),
                processed_message.as_osc(reg.clone()),
            );
        })
        .on_message("/play_sample", &|msg| {
            let processed_message = PlaySampleMessage::new(&msg).unwrap();
            let internal_msg = processed_message.with_buffer_arg(
                sample_dict_arc.clone()
            );
            SCProcessManager::send_timed_packets(
                sc_arc.clone(),
                internal_msg.as_osc(reg.clone()),
            );
        })
        .on_message("/note_modify", &|msg| {
            let processed_message = NoteModifyMessage::new(&msg).unwrap();
            SCProcessManager::send_timed_packets(
                sc_arc.clone(),
                processed_message.as_osc(reg.clone()),
            );
        })
        .on_message("/read_scd", &|msg| {
            sc_arc.lock().unwrap().send_to_client(msg);
        })
        .on_tbundle("nrt_record", &|tagged_bundle| {
            let nrt_record_msg = NRTRecordMessage::from_bundle(tagged_bundle);

            match nrt_record_msg {
                Ok(nrt_record) => {
                    let nrt_result = nrt_record::get_nrt_record_scd(
                        &nrt_record, sample_dict_arc.clone(),
                    ).unwrap();

                    sc_arc.lock().unwrap().send_to_client(
                        OscMessage {
                            addr: "/read_scd".to_string(),
                            args: vec![OscType::String(nrt_result)],
                        }
                    );

                    // TODO: Do something with the /nrt_done message
                }
                Err(e) => { warn!("{}", e) }
            }
        })
        .funnel_tbundle("batch-send")
        .begin();
}
