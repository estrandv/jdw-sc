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
use rosc::{OscMessage, OscPacket, OscType};
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
mod osc_daemon;


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

    let sc_process_data = sc_process_management::init().unwrap_or_else(|err| {
        error!("ERROR BOOTING SUPERCOLLIDER: {:?}", err);
        exit(0)
    });

    let client = sc_process_data.client;

    let process_arc_interrupt = Arc::new(Mutex::new(sc_process_data.process));
    let process_arc_failure = process_arc_interrupt.clone();

    // Terminate supercollider on ctrl+c
    ctrlc::set_handler(move || {
        info!("Thread abort requested");
        process_arc_interrupt.clone().lock().unwrap().terminate().unwrap();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    /*
        Wait for the custom /init message from the server (see start_server.scd.template).
     */
    match client.await_response(
        "/init",
        vec![OscType::String("ok".to_string())],
        Duration::from_secs(10),
    ) {
        Err(e) => {
            error!("{}", e);
            process_arc_failure.lock().unwrap().terminate().unwrap();
        }
        Ok(()) => ()
    };

    info!("Server online!");

    // TODO: Sampler synth must be created on startup - could potentially be part of boot script!

    let sample_pack_dict = SamplePackDict::new();

    let mut sample_pack_dir = home_dir().unwrap();
    sample_pack_dir.push("sample_packs");

    // Populated via osc messages, used e.g. for NRT recording
    let mut loaded_synthdef_snippets: Vec<String> = Vec::new();

    let node_reg = Arc::new(Mutex::new(NodeIDRegistry::new()));


    // Ready the sampler synth - similar to a create_synthdef call.
    let sampler_def = scd_templating::read_scd_file("sampler.scd");
    loaded_synthdef_snippets.push(sampler_def.clone());
    client.send_to_client(OscMessage {
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
        client.send_timed_packets(0, beep(i, node_reg.clone()));
        sleep(Duration::from_millis(125));
    }

    let reg = Arc::new(Mutex::new(NodeIDRegistry::new()));

    info!("Startup completed, polling for messages ...");

    osc_daemon::run(config::get_addr(config::APPLICATION_IN_PORT), client, sample_pack_dict, loaded_synthdef_snippets);


}
