#![feature(result_flattening)]

use std::f32;
use std::process::exit;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use bigdecimal::BigDecimal;
use home::home_dir;
use jdw_osc_lib::model::{OscArgHandler, TaggedBundle, TimedOSCPacket};
use jdw_osc_lib::osc_stack::OSCStack;
use json::Array;
use log::{debug, error, info, LevelFilter, warn};
use regex::Replacer;
use rosc::{OscMessage, OscPacket, OscType};
use simple_logger::SimpleLogger;
use subprocess::{Exec, Popen, PopenConfig, Redirection};

use crate::config::APPLICATION_IN_PORT;
use crate::internal_osc_conversion::SuperColliderMessage;
use crate::node_lookup::NodeIDRegistry;
use crate::nrt_record::NRTConvert;
use crate::osc_model::{LoadSampleMessage, NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, NRTRecordMessage, PlaySampleMessage};
use crate::sampling::SamplePackDict;
use crate::scd_templating::create_nrt_script;
use crate::supercollider::SCProcessManager;

mod supercollider;
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

    - CREATE_SYNTH
        - See dev_diary for explanation why it has to be synth specifically 
        - Note existing code: 
            - All synths code files are read and templated in the same call, which happens twice (startup and NRT)
            - Much like for the sample dict, we should have a struct containing all loaded templates at all times
            - This struct can then export arrays when needed and be supplied to NRT record
            - Calls to /create_synth should immediately add the supplied template
            - I SAY TEMPLATE BUT WE SHOULD DITCH THAT 
                - :synth_name can be manually supplied and has little bearing when files are not the basis anymore 
                - :operation is just the end-part of the file, arguably not part of the synthDef either, so we can just append 

    - LOAD_SAMPLE
        - Also has some background in dev_diary
        - Basically, we can add one sample file at a time by supplying the right paramteters 
            - sample_pack: Add to this pack, create it if not yet present 
            - index: This is sample number <index> in the given pack 
            - path (absolute): Read the sample file from here 
            - category: Group sample in this category
        - Note that existing structures (sample dict) should be able to acocmodate this without any radical changes 
            -> Which also means it should work just fine with NRT 

    - CREATE_EFFECT 
        - Note the above! An effect is essentially just a synth that routes sound from one buffer to another 
        - So this would go under CREATE_SYNTH but with a slightly different template supplied 
        - The neat thing of course being that this handles NRT as well! 
        - Remember: Effects are a bit special in terms of group id and placement in chain 
        
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

            // TODO: Now only uses new method

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


            // TODO: Now implemented with custom load methods - clean all dead code before making the below more readable! 

            let sample_rows: Vec<String> = sample_pack_dict_arc.lock().unwrap()
                .get_all_samples().iter().map(|sample| sample.get_nrt_scd_row())
                .collect();

            let synthdefs: Vec<String> = synthdef_snippets_arc.lock().unwrap().iter().map(|def| def.clone() + ".asBytes").collect();

            // TODO: Compat reading of the scd dir - should eventually be removed in favour of "synthdef_scds".
            let mut scd_rows: Vec<_> = synthdefs.iter()
                .map(|def| { return scd_templating::nrt_wrap_synthdef(def); })
                .collect();

            for s in sample_rows {
                scd_rows.push(s);
            }

            // TODO: Implementing the message rows as above - skipping error handling of from_bundle...

            let nrt_record_msg = NRTRecordMessage::from_bundle(tagged_bundle).unwrap();

            let registry = NodeIDRegistry::new();
            let reg_handle = Arc::new(Mutex::new(registry));

            let mut current_beat = BigDecimal::from_str("0.0").unwrap();

            let message_rows: Vec<String> = nrt_record_msg.messages.iter()
                .flat_map(|timed_packet| {
                    let osc = internal_osc_conversion::resolve_msg(timed_packet.packet.clone(), sample_pack_dict_arc.clone())
                        .as_nrt_osc(reg_handle.clone(), current_beat.clone());
                    current_beat += timed_packet.time.clone();
                    osc
                })
                .map(|osc| osc.as_nrt_row())
                .collect();

            for m in message_rows {
                scd_rows.push(m);
            }

            let script = create_nrt_script(
                nrt_record_msg.bpm,
                &nrt_record_msg.file_name,
                nrt_record_msg.end_beat,
                scd_rows,
            );

            sc_arc.lock().unwrap().send_to_client(
                OscMessage {
                    addr: "/read_scd".to_string(),
                    args: vec![OscType::String(script.unwrap())],
                }
            );

            // TODO: Do something with the /nrt_done message
        })
        // Treat each packet in batch-send as a separately interpreted packet
        .funnel_tbundle("batch-send")
        .begin();
}
