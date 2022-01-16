mod supercollider;
mod zeromq;
mod model;
mod synth_templates;

use subprocess::{Exec, Redirection, Popen, PopenConfig};
use std::process::exit;
use std::sync::{Mutex, Arc};
use crate::supercollider::{Supercollider, NodeManager};
use rosc::{OscType, OscMessage};
use std::cell::RefCell;
use crate::zeromq::ZMQSubscriber;
use crate::model::{ProscNoteCreateMessage, ProscNoteModifyMessage};

fn main() {
    println!("Hello, world!");

    let handler = Supercollider::new();
    let arc = Arc::new(Mutex::new(handler));
    let arc_in_ctrlc = arc.clone();

    let thread_abort = Arc::new(Mutex::new(RefCell::new(false)));
    let ctrlc_thread_abort = thread_abort.clone();

    ctrlc::set_handler(move || {
        ctrlc_thread_abort.lock().unwrap().replace(true);
        println!("Thread abort requested");
        arc_in_ctrlc.lock().unwrap().terminate();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    // NOTE: this also prevents, ctrl+c due to the lock
    arc.lock().unwrap().wait_for("/init", vec![OscType::String("ok".to_string())], thread_abort.clone());

    println!("Server online!");

    let sc_client = NodeManager::new(arc.clone());

    let synth_defs = synth_templates::read_all("add");

    for def in synth_defs {
        arc.lock().unwrap().send_to_client(
            OscMessage {
                addr: "/read_scd".to_string(),
                args:  vec![OscType::String(def)]
            }
        )
    }

    // Send hello ping
    sc_client.s_new_timed_gate(
        "default",
        vec![OscType::String("freq".to_string()), OscType::Float(240.0)],
        0.1
    );

    let sc_loop_client = Arc::new(Mutex::new(sc_client));

    let subscriber = ZMQSubscriber::new();

    // Read incoming messages from ZMQ queue in loop
    loop {
        println!("DEBUG: Loop reset");
        let msg = subscriber.recv();
        println!("DEBUG: Message inc {}", &msg.msg_type);

        if msg.msg_type == String::from("JDW.ADD.NOTE") {

            println!("INcoming note on");
            let payload: ProscNoteCreateMessage = serde_json::from_str(&msg.json_contents).unwrap();

            sc_loop_client.lock().unwrap()
                .s_new(
                    &payload.external_id,
                    &payload.target,
                    payload.get_arg_vec()
                );


        } else if msg.msg_type == String::from("JDW.NSET.NOTE") {
            let payload: ProscNoteModifyMessage = serde_json::from_str(&msg.json_contents).unwrap();

            sc_loop_client.lock().unwrap()
                .note_mod(
                    &payload.external_id,
                    payload.get_arg_vec()
                );


        } else if msg.msg_type == String::from("JDW.RMV.NOTE") {

        } else {
            panic!("Unknown message type: {}", msg.msg_type);
        }
    }

}
