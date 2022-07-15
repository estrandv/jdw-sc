#![feature(result_flattening)]

mod supercollider;
mod scd_templating;
mod samples;
mod osc_model;
mod osc_client;
mod nrt_record;
mod config;
mod internal_osc_conversion;

use subprocess::{Exec, Redirection, Popen, PopenConfig};
use std::process::exit;
use std::sync::{Mutex, Arc};
use crate::supercollider::{Supercollider};
use rosc::{OscType, OscMessage, OscPacket};
use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;
use log::{debug, error, info, LevelFilter, warn};
use simple_logger::SimpleLogger;
use crate::internal_osc_conversion::{IdRegistry, InternalOSCMorpher};
use crate::osc_client::OSCPoller;
use crate::osc_model::{PlaySampleMessage, NoteOnTimedMessage, NoteModifyMessage, NoteOnMessage, TaggedBundle};
use crate::samples::SampleDict;

fn main() {

    // Handles all log macros, e.g. "warn!()" to print info in terminal
    SimpleLogger::new()
        .with_level(config::LOG_LEVEL)
        .init().unwrap();

    /*
        Prepare thread handler for the main supercollider instance
     */
    let handler = Supercollider::new();
    let arc = Arc::new(Mutex::new(handler));
    let arc_in_ctrlc = arc.clone();

    // Terminate supercollider on ctrl+c
    ctrlc::set_handler(move || {
        info!("Thread abort requested");
        arc_in_ctrlc.lock().unwrap().terminate();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    /*
        Wait for the custom /init message from the server (see start_server.scd).
        TODO: Does the application crash on timeout? Some kind of handling/termination is needed.
     */
    arc.lock().unwrap()
        .wait_for("/init", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

    info!("Server online!");

    /*
        Use the synth definitions from the synths dir to ready custom scd messages.
        The messages then create these synthdefs on the server using the sclang client.
     */
    let synth_defs = scd_templating::read_all_synths("add");

    // See start_server.scd for the /read_scd definition
    for def in synth_defs {
        arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args:  vec![OscType::String(def)]
            }
        )
    }

    /*
        Prepare sample players. All samples are read into buffers via read_scd on the sclang client.
        The sample dict struct keeps track of which buffer index belongs to which sample pack.
     */
    let buffer_data = samples::SampleDict::from_dir(Path::new("sample_packs")).unwrap_or_else(|e| {
        error!("Unable to read buffer data: {} - no samples will be provided", e);
        SampleDict::dummy()
    });

    let buffer_handle = Arc::new(Mutex::new(buffer_data));
    let buffer_string = buffer_handle.clone().lock().unwrap().to_buffer_load_scd();

    if !buffer_string.is_empty() {

        arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args:  vec![OscType::String(buffer_string)]
            }
        );

        // Message is added to the end of the buffer load scd to signify a completed load call.
        // TODO: Does this trigger? Is it accurate? There was a note previously to remove it. Also error handling...
        arc.lock().unwrap().wait_for("/buffers_loaded", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

    }

    ///////////////////////////


    let node_reg = Arc::new(Mutex::new(IdRegistry::new()));

    // Play a welcoming ping in a really obtuse way.
    Supercollider::send_timed(arc.clone(),
        NoteOnTimedMessage::new(OscMessage {
            addr: "/note_on_timed".to_string(),
            args: vec![
                OscType::String("gentle".to_string()),
                OscType::String("launch_ping".to_string()),
                OscType::Float(0.5),
                OscType::String("freq".to_string()),
                OscType::Float(240.0)
            ]
        }).unwrap().as_osc(node_reg.clone())
    );

    let mut osc_poller = OSCPoller::new();

    let main_loop = MainLoop {
        sc_loop_client: arc,
        buffer_handle,
        node_registry: Arc::new(Mutex::new(IdRegistry::new()))
    };

    info!("Startup completed, polling for messages ...");

    loop {

        // TODO: Unless all operations are lightning-fast there might be a need for a poller/processor pattern
        // E.g. one thread polls and fills a buffer, the other eats through said buffer
        // THis might also be relevant for the router
        match osc_poller.poll() {
            Ok(packet) => {
                main_loop.process_osc(packet);
            }
            Err(e_str) => {
                warn!("{}", &e_str);
            }
        };

    }

    struct MainLoop {
        sc_loop_client: Arc<Mutex<Supercollider>>,
        buffer_handle: Arc<Mutex<SampleDict>>,
        node_registry: Arc<Mutex<IdRegistry>>,
    }

    impl MainLoop {

        fn handle_message(&self, msg: OscMessage) -> Result<(), String> {
            // Handle with result to bring down duplicate code below

            if msg.addr == "/note_on_timed" {

                let processed_message = NoteOnTimedMessage::new(msg)?;
                Supercollider::send_timed(
                    self.sc_loop_client.clone(),
                    processed_message.as_osc(self.node_registry.clone())
                );

                Ok(())

            }
            else if msg.addr == "/note_on" {
                let processed_message = NoteOnMessage::new(msg)?;
                Supercollider::send_timed(
                    self.sc_loop_client.clone(),
                    processed_message.as_osc(self.node_registry.clone())
                );

                Ok(())
            }
            else if msg.addr == "/play_sample" {

                let processed_message = PlaySampleMessage::new(msg)?;
                let internal_msg = processed_message.into_internal(
                    self.buffer_handle.clone()
                );
                Supercollider::send_timed(
                    self.sc_loop_client.clone(),
                    internal_msg.as_osc(self.node_registry.clone())
                );

                Ok(())
            }
            else if msg.addr == "/note_modify" {
                let processed_message = NoteModifyMessage::new(msg)?;
                Supercollider::send_timed(
                    self.sc_loop_client.clone(),
                    processed_message.as_osc(self.node_registry.clone())
                );


                Ok(())
            }
            else {

                // TODO: ... each unknown address will be forwarded straight to sc
                // Main loop does not have a direct handle of supercollider.send_to_server...
                // It only has a nodemanager... which has a handle.
                // Might be worth rethinking what does what in supercollider.rs
                // ... but in the meantime this is not an important feature
                // Also note: might there be client messages we want to send from outside?

                Ok(())
            }

        }

        fn process_osc(
            &self,
            packet: OscPacket
        ) {
            match packet {
                OscPacket::Message(msg) => {

                    debug!(">> Received OSC message for function/address: {} with args {:?}", msg.addr, msg.args);

                    match self.handle_message(msg) {
                        Ok(()) => {}
                        Err(e) => {warn!("{}", e)}
                    };

                }
                OscPacket::Bundle(bundle) => {

                    debug!("OSC Bundle: {:?}", bundle);

                    match TaggedBundle::new(bundle) {
                        Ok(tagged_bundle) => {
                            info!("Parse of bundle successful: {:?}", tagged_bundle);

                            if tagged_bundle.bundle_tag == "batch_send" {
                                for sub_packet in tagged_bundle.contents {
                                    debug!("Unpacking: {:?}", sub_packet.clone());
                                    self.process_osc(sub_packet);
                                }
                            }

                        }
                        Err(e) => {
                            warn!("{}", e);
                        }
                    }
                }
            }
        }
    }




}
