#![feature(result_flattening)]

use std::cell::RefCell;
use std::path::Path;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use home::home_dir;
use jdw_osc_lib::model::TaggedBundle;
use jdw_osc_lib::osc_stack::OSCStack;
use log::{debug, error, info, LevelFilter, warn};
use rosc::{OscMessage, OscPacket, OscType};
use simple_logger::SimpleLogger;
use subprocess::{Exec, Popen, PopenConfig, Redirection};

use crate::config::APPLICATION_IN_PORT;
use crate::internal_osc_conversion::{IdRegistry, InternalOSCMorpher};
use crate::osc_model::{NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, NRTRecordMessage, PlaySampleMessage};
use crate::samples::SampleDict;
use crate::scd_templating::create_nrt_script;
use crate::supercollider::Supercollider;

mod supercollider;
mod scd_templating;
mod samples;
mod osc_model;
mod nrt_record;
mod config;
mod internal_osc_conversion;

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
    let synth_defs = scd_templating::read_all_synths("add;");

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

    let mut home_dir = home_dir().unwrap();
    home_dir.push("sample_packs");

    let buffer_data = samples::SampleDict::from_dir(&home_dir).unwrap_or_else(|e| {
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
        NoteOnTimedMessage::new(&OscMessage {
            addr: "/note_on_timed".to_string(),
            args: vec![
                OscType::String("default".to_string()),
                OscType::String("launch_ping".to_string()),
                OscType::String("0.5".to_string()),
                OscType::String("freq".to_string()),
                OscType::Float(240.0)
            ]
        }).unwrap().as_osc(node_reg.clone())
    );

    let reg = Arc::new(Mutex::new(IdRegistry::new()));

    info!("Startup completed, polling for messages ...");

    // TODO: Problem with "batch-send" tagged bundle (handle each contained message as incoming message9
    //
    OSCStack::init(config::get_addr(APPLICATION_IN_PORT))
        .on_message("/note_on_timed", &|msg| {
            let processed_message = NoteOnTimedMessage::new(&msg).unwrap();
            Supercollider::send_timed(
                arc.clone(),
                processed_message.as_osc(reg.clone())
            );
        })
        .on_message("/note_on", &|msg| {
            let processed_message = NoteOnMessage::new(&msg).unwrap();
            Supercollider::send_timed(
                arc.clone(),
                processed_message.as_osc(reg.clone())
            );
        })
        .on_message("/play_sample", &|msg| {
            let processed_message = PlaySampleMessage::new(&msg).unwrap();
            let internal_msg = processed_message.into_internal(
                buffer_handle.clone()
            );
            Supercollider::send_timed(
                arc.clone(),
                internal_msg.as_osc(reg.clone())
            );
        })
        .on_message("/note_modify", &|msg| {
            let processed_message = NoteModifyMessage::new(&msg).unwrap();
            Supercollider::send_timed(
                arc.clone(),
                processed_message.as_osc(reg.clone())
            );
        })
        .on_message("/read_scd", &|msg| {
            arc.lock().unwrap().send_to_client(msg);
        })
        .on_tbundle("nrt_record", &|tagged_bundle| {
            let nrt_record_msg = NRTRecordMessage::from_bundle(tagged_bundle);

            match nrt_record_msg {
                Ok(nrt_record) => {

                    let nrt_result = nrt_record::get_nrt_record_scd(
                        &nrt_record, buffer_handle.clone()
                    ).unwrap();

                    //println!("NRT\n\n: {}", &nrt_result);

                    arc.lock().unwrap().send_to_client(
                        OscMessage {
                            addr: "/read_scd".to_string(),
                            args:  vec![OscType::String(nrt_result)]
                        }
                    );

                    // TODO: waiting works but is of course disruptive
                    // (Tested: it is.)
                    // What we do want eventually however is some kind of
                    // "execute on message" that sends out a message to the
                    // router that the file is created and exists at a path

                    //self.sc_loop_client.lock().unwrap()
                    //    .wait_for("/nrt_done", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

                }
                Err(e) => {warn!("{}", e)}
            }
        })
        .begin();

}
