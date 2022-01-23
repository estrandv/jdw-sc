mod supercollider;
mod zeromq;
mod model;
mod synth_templates;
mod samples;

use subprocess::{Exec, Redirection, Popen, PopenConfig};
use std::process::exit;
use std::sync::{Mutex, Arc};
use crate::supercollider::{Supercollider, NodeManager};
use rosc::{OscType, OscMessage};
use std::cell::RefCell;
use crate::zeromq::{ZMQSubscriber, into_message, ZMQMsg};
use crate::model::{ProscNoteCreateMessage, ProscNoteModifyMessage, JdwPlayNoteMsg, JdwPlaySampleMsg, JdwSequencerBatchMsg};
use std::path::Path;
use std::time::Duration;
use crate::samples::SampleDict;

fn main() {
    println!("Hello, world!");

    let handler = Supercollider::new();
    let arc = Arc::new(Mutex::new(handler));
    let arc_in_ctrlc = arc.clone();

    ctrlc::set_handler(move || {
        println!("Thread abort requested");
        arc_in_ctrlc.lock().unwrap().terminate();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    // NOTE: this also prevents, ctrl+c due to the lock
    arc.lock().unwrap().wait_for("/init", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

    println!("Server online!");

    let sc_client = NodeManager::new(arc.clone());

    // Load synths and buffers
    let synth_defs = synth_templates::read_all("add");

    for def in synth_defs {
        arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args:  vec![OscType::String(def)]
            }
        )
    }

    let buffer_data = samples::SampleDict::from_dir(Path::new("sample_packs"));

    let buffer_handle = Arc::new(Mutex::new(buffer_data));

    let buffer_string = buffer_handle.clone().lock().unwrap().to_buffer_load_scd();

    if !buffer_string.is_empty() {

        arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args:  vec![OscType::String(buffer_string)]
            }
        );

        // Not needed for hello ping but neat until we have proper wait times for everything // TODO: Remove
        arc.lock().unwrap().wait_for("/buffers_loaded", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

        // Do the same for buffer
        sc_client.sample_trigger(vec![]);

    }

    ///////////////////////////

    // Send hello ping
    sc_client.s_new_timed_gate(
        "default",
        vec![OscType::String("freq".to_string()), OscType::Float(240.0)],
        0.1
    );

    let sc_loop_client = Arc::new(Mutex::new(sc_client));

    let subscriber = ZMQSubscriber::new();

    let main_loop = MainLoop {
        sc_loop_client,
        buffer_handle
    };

    // Read incoming messages from ZMQ queue in loop
    loop {
        let recv_msg = subscriber.recv();

        main_loop.process_msg(recv_msg);

    }

    struct MainLoop {
        sc_loop_client: Arc<Mutex<NodeManager>>,
        buffer_handle: Arc<Mutex<SampleDict>>,
    }

    impl MainLoop {

        // Place message process in struct to allow recursive calls (batch) with dependencies
        fn process_msg(
            &self,
            msg: ZMQMsg
        ) {
            if msg.msg_type == String::from("JDW.ADD.NOTE") {

                // Add note with no explicit end time. Typically requires gate mod to turn off.

                println!("INcoming note on");
                let payload: ProscNoteCreateMessage = serde_json::from_str(&msg.json_contents).unwrap();

                match payload.get_gate_time() {
                    Some(time) => {
                        self.sc_loop_client.lock().unwrap()
                            .s_new_timed_gate(
                                &payload.target,
                                payload.get_arg_vec(),
                                time
                            );
                    },
                    None => {
                        self.sc_loop_client.lock().unwrap()
                            .s_new(
                                &payload.external_id,
                                &payload.target,
                                payload.get_arg_vec()
                            );
                    }
                }

            } else if msg.msg_type == String::from("JDW.NSET.NOTE") {

                // Any changing of sc args, including the "note off" gate arg

                let payload: ProscNoteModifyMessage = serde_json::from_str(&msg.json_contents).unwrap();

                self.sc_loop_client.lock().unwrap()
                    .note_mod(
                        &payload.external_id,
                        payload.get_arg_vec()
                    );


            } else if msg.msg_type == String::from("JDW.PLAY.SAMPLE") {
                let payload: JdwPlaySampleMsg = serde_json::from_str(&msg.json_contents).unwrap();

                self.sc_loop_client.lock().unwrap()
                    .sample_trigger(
                        payload.get_arg_vec(self.buffer_handle.clone())
                    );

            } else if msg.msg_type == String::from("JDW.SEQ.BATCH") {

                /*
                    TODO: Decoding the batch
                    - Each sequencer tick has a message that is a jdw message in plainstring: "JDW.BLA::1991::{"args": ...}"
                    - Batch wraps a list of these as the json: JDW.BATCH::["blabla", "bla"]
                    - As such, decoding JDW.BATCH is mainly about taking the json part in as Vec<String> and then running
                        message decodes from there
                 */

                println!("{:?}", &msg);

                let vector_payload: Vec<String> = serde_json::from_str(&msg.json_contents).unwrap();

                for batch_msg in vector_payload {
                    let dec_batch_msg = into_message(&batch_msg);
                    // Note recursion
                    self.process_msg(dec_batch_msg);
                }

            } else if msg.msg_type == String::from("JDW.PLAY.NOTE") {

                // Auto-gated, typical "sequencer" note play

                let payload: JdwPlayNoteMsg = serde_json::from_str(&msg.json_contents).unwrap();
                self.sc_loop_client.lock().unwrap()
                    .s_new_timed_gate(
                        &payload.target,
                        payload.get_arg_vec(),
                        payload.get_gate_time()
                    );
            } else {
                println!("Unknown message type: {}", msg.msg_type);
            }
        }
    }




}
