mod processes;

use subprocess::{Exec, Redirection, Popen, PopenConfig};
use std::process::exit;
use std::sync::{Mutex, Arc};
use crate::processes::{ProcessHandler, Supercollider};
use rosc::OscType;

fn main() {
    println!("Hello, world!");

    let handler = Supercollider::new();
    let arc = Arc::new(Mutex::new(handler));
    let arc_in_ctrlc = arc.clone();

    ctrlc::set_handler(move || {
        arc_in_ctrlc.lock().unwrap().terminate();
        exit(0);
    }).expect("Error setting Ctrl-C handler");

    arc.lock().unwrap().wait_for("/init", vec![OscType::String("ok".to_string())]);

    println!("Server online!");

    loop {

        if !arc.lock().unwrap().is_alive() {
            // the process has finished
            println!("Done!");
            exit(0);
        } else {
            //p.terminate();
        }
    }

}
