#![feature(result_flattening)]

pub mod config;
pub mod internal_osc_conversion;
pub mod node_lookup;
pub mod nrt_record;
pub mod osc_daemon;
pub mod osc_model;
pub mod sampling;
pub mod sc_process_management;
pub mod scd_templating;

use crate::internal_osc_conversion::SuperColliderMessage;
use crate::node_lookup::NodeIDRegistry;
use crate::osc_model::NoteOnTimedMessage;
use home::home_dir;
use jdw_osc_lib::model::TimedOSCPacket;
use log::{error, info};
use rosc::{OscMessage, OscType};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

/// Run the jdw-sc daemon. Blocks the calling thread indefinitely.
///
/// * `config_path` – path to the per-app `config.toml`.
/// * `quiet`       – suppress non-error log output.
pub fn run(config_path: &str, quiet: bool) {
    config::init(config_path);

    // Logging must only be initialised once per process; in library mode the
    // caller is responsible. We attempt init here for the standalone-binary
    // case and silently ignore the "already initialised" error.
    let _ = simple_logger::SimpleLogger::new()
        .with_level(if quiet {
            log::LevelFilter::Error
        } else {
            config::Config::get().log_level_filter()
        })
        .init();

    let sc_process_data = sc_process_management::init().unwrap_or_else(|err| {
        error!("ERROR BOOTING SUPERCOLLIDER: {:?}", err);
        exit(0)
    });

    let client = sc_process_data.client;

    let process_arc_interrupt = Arc::new(Mutex::new(sc_process_data.process));
    let process_arc_failure = process_arc_interrupt.clone();

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

    match client.await_internal_response(
        "/init",
        vec![OscType::String("ok".to_string())],
        Duration::from_secs(config::Config::get().init_wait_timeout_secs),
    ) {
        Err(e) => {
            error!("{}", e);
            process_arc_failure.lock().unwrap().terminate().unwrap();
        }
        Ok(()) => (),
    };

    info!("Server online!");

    let mut sample_pack_dir = home_dir().unwrap();
    sample_pack_dir.push(
        config::Config::get()
            .sample_pack_dir
            .trim_start_matches("~/"),
    );

    let node_reg = Arc::new(Mutex::new(NodeIDRegistry::new()));

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
                OscType::String("launch_ping_{nodeId}".to_string()),
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

    for i in [130.81, 146.83, 196.00] {
        client.send_timed_packets_to_scsynth(0, beep(i, node_reg.clone()), SystemTime::now());
        sleep(Duration::from_millis(125));
    }

    info!("Startup completed, polling for messages ...");

    osc_daemon::run(
        config::get_addr(config::Config::get().application_in_port),
        client,
        sampler_def,
    );
}
