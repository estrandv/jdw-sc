mod supercollider;

use subprocess::{Exec, Redirection, Popen, PopenConfig};
use std::process::exit;
use std::sync::{Mutex, Arc};
use crate::supercollider::{Supercollider, NodeManager};
use rosc::{OscType, OscMessage};
use std::cell::RefCell;

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

    sc_client.s_new_timed_gate(
        "default",
        vec![OscType::String("freq".to_string()), OscType::Float(240.0)],
        0.1
    );

    loop {
        // stay alive
    }

}
