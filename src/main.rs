#![feature(result_flattening)]

use crate::internal_osc_conversion::SuperColliderMessage;
use crate::node_lookup::NodeIDRegistry;
use crate::osc_model::NoteOnTimedMessage;
use home::home_dir;
use jdw_osc_lib::model::TimedOSCPacket;
use log::{error, info};
use rosc::{OscMessage, OscType};
use simple_logger::SimpleLogger;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

mod config;
mod internal_osc_conversion;
mod node_lookup;
mod nrt_record;
mod osc_daemon;
mod osc_model;
mod sampling;
mod sc_process_management;
mod scd_templating;

fn main() {
    // Handles all log macros, e.g. "warn!()" to print info in terminal
    SimpleLogger::new()
        .with_level(config::LOG_LEVEL)
        .init()
        .unwrap();

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
        process_arc_interrupt
            .clone()
            .lock()
            .unwrap()
            .terminate()
            .unwrap();
        exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    /*
       Wait for the custom /init message from the server (see start_server.scd.template).
    */
    match client.await_internal_response(
        "/init",
        vec![OscType::String("ok".to_string())],
        Duration::from_secs(10),
    ) {
        Err(e) => {
            error!("{}", e);
            process_arc_failure.lock().unwrap().terminate().unwrap();
        }
        Ok(()) => (),
    };

    info!("Server online!");

    let mut sample_pack_dir = home_dir().unwrap();
    sample_pack_dir.push("sample_packs");

    let node_reg = Arc::new(Mutex::new(NodeIDRegistry::new()));

    // Ready the sampler synth - similar to a create_synthdef call (which is why we let the interpreter know about it, too).
    let sampler_def = scd_templating::read_scd_file("sampler.scd");
    client.send_to_sclang(OscMessage {
        addr: "/read_scd".to_string(),
        args: vec![OscType::String(sampler_def.clone() + ".add;")],
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
        })
        .unwrap()
        .as_osc(node_reg)
    }

    // Play a welcoming tune in a really obtuse way.
    for i in [130.81, 146.83, 196.00] {
        client.send_timed_packets_to_scsynth(0, beep(i, node_reg.clone()));
        sleep(Duration::from_millis(125));
    }

    info!("Startup completed, polling for messages ...");

    osc_daemon::run(
        config::get_addr(config::APPLICATION_IN_PORT),
        client,
        sampler_def,
    );
}
