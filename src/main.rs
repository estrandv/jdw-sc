mod supercollider;
mod zeromq;
mod model;
mod synth_templates;
mod samples;
mod osc_model;
mod osc_client;

use subprocess::{Exec, Redirection, Popen, PopenConfig};
use std::process::exit;
use std::sync::{Mutex, Arc};
use crate::supercollider::{Supercollider, NodeManager};
use rosc::{OscType, OscMessage, OscPacket};
use std::cell::RefCell;
use crate::zeromq::{ZMQSubscriber, into_message, ZMQMsg};
use crate::model::{ProscNoteCreateMessage, ProscNoteModifyMessage, JdwPlayNoteMsg, JdwPlaySampleMsg, JdwSequencerBatchMsg};
use std::path::Path;
use std::time::Duration;
use crate::osc_client::OSCPoller;
use crate::osc_model::{PlaySampleMessage, NoteOnTimedMessage, NoteModifyMessage, NoteOnMessage};
use crate::samples::SampleDict;

fn main() {

    /*
        Prepare thread handler for the main supercollider instance
     */
    let handler = Supercollider::new();
    let arc = Arc::new(Mutex::new(handler));
    let arc_in_ctrlc = arc.clone();

    // Terminate supercollider on ctrl+c
    ctrlc::set_handler(move || {
        println!("Thread abort requested");
        arc_in_ctrlc.lock().unwrap().terminate();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    /*
        Wait for the custom /init message from the server (see start_server.scd).
        TODO: Does the application crash on timeout? Some kind of handling/termination is needed.
     */
    arc.lock().unwrap()
        .wait_for("/init", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

    println!("Server online!");

    let sc_client = NodeManager::new(arc.clone());

    /*
        Use the synth definitions from the synths dir to ready custom scd messages.
        The messages then create these synthdefs on the server using the sclang client.
     */
    let synth_defs = synth_templates::read_all("add");

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

        // Message is added to the end of the buffer load scd to signify a completed load call.
        // TODO: Does this trigger? Is it accurate? There was a note previously to remove it. Also error handling...
        arc.lock().unwrap().wait_for("/buffers_loaded", vec![OscType::String("ok".to_string())], Duration::from_secs(10));

    }

    ///////////////////////////

    /*
        Play some welcoming sounds.
     */

    // Play a default sample to notify the user that samples are live.
    // Note how an empty arg-array will simply play the first loaded buffer.
    sc_client.sample_trigger(vec![]);

    sc_client.note_on_timed(
        "default",
        "initial_testnote",
        vec![OscType::String("freq".to_string()), OscType::Float(240.0)],
        0.1
    );


    // Create a thread handle for the main loop.
    let sc_loop_client = Arc::new(Mutex::new(sc_client));

    // TODO: Replace with OSC
    let zmq_subscriber = ZMQSubscriber::new();
    let mut osc_poller = OSCPoller::new();

    let main_loop = MainLoop {
        sc_loop_client,
        buffer_handle
    };

    println!("Startup completed, polling for messages ...");

    // Read incoming messages from ZMQ queue in loop
    loop {

        // TODO: Unless all operations are lightning-fast there might be a need for a poller/processor pattern
        // E.g. one thread polls and fills a buffer, the other eats through said buffer
        // THis might also be relevant for the router
        match osc_poller.poll() {
            Ok(packet) => {
                main_loop.process_osc(packet);
            }
            Err(e_str) => {
                println!("{}", &e_str);
            }
        };

        //let recv_msg = zmq_subscriber.recv();
    }

    struct MainLoop {
        sc_loop_client: Arc<Mutex<NodeManager>>,
        buffer_handle: Arc<Mutex<SampleDict>>,
    }

    impl MainLoop {

        // TODO: Starting a side-implementation for easier switch to OSC
        fn process_osc(
            &self,
            packet: OscPacket
        ) {
            match packet {
                OscPacket::Message(msg) => {

                    println!(">> Received OSC message for function/address: {} with args {:?}", msg.addr, msg.args);

                    // TODO: Each known address will have an osc_model object it can be parsed into

                    if msg.addr == "/note_on_timed" {
                        match NoteOnTimedMessage::new(msg) {
                            Ok(processed_message) => {
                                // TODO: Provide external id as well
                                self.sc_loop_client.lock().unwrap()
                                    .note_on_timed(
                                        &processed_message.synth_name,
                                        &processed_message.external_id,
                                        processed_message.args,
                                        processed_message.gate_time
                                    );
                            },
                            Err(err_msg) => {
                                println!("Error processing incoming osc: {}", &err_msg);
                            }
                        }
                    }
                    else if msg.addr == "/note_on" {
                        match NoteOnMessage::new(msg) {
                            Ok(processed_message) => {
                                self.sc_loop_client.lock().unwrap()
                                    .note_on(
                                        &processed_message.external_id,
                                        &processed_message.synth_name,
                                        processed_message.args,
                                    );
                            },
                            // TODO: Same for all, could be restructured as a combined ? call
                            Err(err_msg) => {
                                println!("Error processing incoming osc: {}", &err_msg);
                            }
                        }
                    }
                    else if msg.addr == "/play_sample" {
                        match PlaySampleMessage::new(msg) {
                            Ok(processed_message) => {
                                self.sc_loop_client.lock().unwrap()
                                    .sample_trigger(
                                        // Note how get_arg_vec constructs different args using sample dict data
                                        processed_message.get_args_with_buf(self.buffer_handle.clone())
                                    );
                            },
                            Err(err_msg) => {
                                println!("Error processing incoming osc: {}", &err_msg);
                            }
                        }
                    }
                    else if msg.addr == "/note_modify" {
                        match NoteModifyMessage::new(msg) {
                            Ok(processed_message) => {

                                self.sc_loop_client.lock().unwrap().note_mod(
                                    &processed_message.external_id_regex,
                                    processed_message.args
                                );

                            },
                            Err(err_msg) => {
                                println!("Error processing incoming osc: {}", &err_msg);
                            }
                        }
                    }
                    else {
                        // TODO: ... each unknown address will be forwarded straight to sc
                    }

                    // TODO: Might there be client messages that we want to send from outside?

                }
                OscPacket::Bundle(bundle) => {

                    // TODO: All incoming bundles will require a bundle_info message to be processed

                    println!("OSC Bundle: {:?}", bundle);
                }
            }
        }

        // TODO: Main reason for struct is to allow recursive calls that the old BATCH method of message grouping required.
        // A similar handle might be wanted for osc bundles but I'm unsure if it would save any time in UDP.
        // Note: We're talking about sequencer sending a hundred tracks all at once, e.g. at loop start for 0.0 notes.
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
                            .note_on_timed(
                                "dummy_note", // Will be removed soon...
                                &payload.target,
                                payload.get_arg_vec(),
                                time
                            );
                    },
                    None => {
                        self.sc_loop_client.lock().unwrap()
                            .note_on(
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
                        // Note how get_arg_vec constructs different args using sample dict data
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
                    .note_on_timed(
                        "dummy_note", // Will be removed soon ...
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
