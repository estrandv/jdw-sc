use subprocess::{Popen, PopenConfig, Redirection};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use rosc::{OscPacket, OscTime, OscType, OscMessage, encoder};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::sync::{Mutex, Arc};
use std::cell::RefCell;

// TODO: KIll twin in model.rs
#[derive(Debug, Clone)]
struct RunningNote {
    pub synth: String,
    pub external_id: String,
    pub args: Vec<OscType>,
    pub node_id: i32,
}

impl RunningNote {

    fn get_arg(&self, arg_name: &str) -> Option<OscType> {
        let pos = self.args.iter().position(|arg| arg.clone() == OscType::String(arg_name.to_string()));

        match pos {
            Some(index) => {
                let value_pos = index + 1;

                match self.args.get(value_pos) {
                    Some(val) => Option::Some(val.clone()),
                    None => None
                }

            },
            None => Option::None
        }

    }

    fn to_s_new(&self) -> OscMessage {
        let mut final_args = vec![
            OscType::String(self.synth.to_string()),
            OscType::Int(self.node_id), // NodeID
            OscType::Int(0), // Group?
            OscType::Int(0), // Group placement?
        ];

        final_args.extend(self.args.clone());

        OscMessage {
            addr: "/s_new".to_string(),
            args:  final_args
        }
    }

    fn to_note_off(&self) -> OscMessage {
        OscMessage {
            addr: "/n_set".to_string(),
            args: vec![
                OscType::Int(self.node_id), // NodeID
                OscType::String("gate".to_string()), // Gate is the "note off" signal
                OscType::Int(0),
            ]
        }
    }

    fn replace_args(&mut self, args: Vec<OscType>) {
        self.args = args;
    }

    fn to_note_mod(&self) -> OscMessage {

        let mut final_args = vec![
            OscType::Int(self.node_id), // NodeID
        ];

        final_args.extend(self.args.clone());

        OscMessage {
            addr: "/n_set".to_string(),
            args: final_args
        }
    }
}


pub struct NodeManager {
    sc_handle: Arc<Mutex<Supercollider>>,
    current_node_id: RefCell<i32>,
    running_notes: Vec<RunningNote>,
}

impl NodeManager {
    pub fn new(sc_handle: Arc<Mutex<Supercollider>>) -> NodeManager {
        NodeManager {
            sc_handle,
            current_node_id: RefCell::new(100),
            running_notes: Vec::new(),
        }
    }

    fn create_note(&self, external_id: &str, synth_name: &str, args: Vec<OscType>) -> RunningNote {
        let current_id = self.current_node_id.clone().into_inner();
        self.current_node_id.replace(current_id + 1);

        RunningNote {
            synth: synth_name.to_string(),
            external_id: external_id.to_string(),
            args,
            node_id: current_id
        }
    }

    fn get_running(&self, external_id: &str) -> Option<RunningNote> {

        let res = self.running_notes.iter().find(|&note| note.external_id == external_id);

        match res {
            Some(element) => Option::Some(element.clone()),
            None => None
        }

    }

    fn remove_running(&mut self, external_id: &str) {
        self.running_notes.retain(|note| note.external_id != external_id);
    }

    pub fn s_new(&mut self, external_id: &str, synth_name: &str, args: Vec<OscType>) {

        let new_note = self.create_note(
            external_id,
            synth_name,
            args
        );

        self.running_notes.push(new_note.clone());

        self.sc_handle.lock().unwrap().send_to_server(new_note.to_s_new());

    }

    pub fn note_mod(&mut self, external_id: &str, args: Vec<OscType>) {
        let running = self.get_running(external_id);

        if running.is_some() {

            let mut moddable = running.unwrap().clone();

            // Full absolute replace - might want relative eventually
            moddable.replace_args(args);

            self.sc_handle.lock().unwrap().send_to_server(moddable.clone().to_note_mod());

            self.remove_running(external_id);

            // Keep track of note off (gate 0)
            let gate_arg = moddable.clone().get_arg("gate");
            let note_off = match gate_arg {
                Some(osc_type) => osc_type == OscType::Float(0.0),
                None => false
            };

            if !note_off {
                // Only re-add the modified note if we didn't turn it off
                self.running_notes.push(moddable);
            }

        }
    }

    pub fn note_off(&mut self, external_id: &str) {
        let running = self.get_running(external_id);

        if running.is_some() {
            self.sc_handle.lock().unwrap().send_to_server(running.unwrap().to_note_off());
            self.remove_running(external_id);
        }
    }

    pub fn s_new_timed_gate(&self, synth_name: &str, args: Vec<OscType>, gate_time_sec: f32) {

        let new_note = self.create_note(
            &format!("{}_dummy", synth_name),
            synth_name,
            args
        );

        self.sc_handle.lock().unwrap().send_to_server(new_note.to_s_new());

        let handle_clone = self.sc_handle.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs_f32(gate_time_sec));
            handle_clone.lock().unwrap().send_to_server(new_note.to_note_off());
        });
    }
}

pub struct Supercollider {
    sclang_process: Popen,
    osc_socket: UdpSocket,
    sclang_out_addr: SocketAddrV4,
    scsynth_out_addr: SocketAddrV4,
}

impl Supercollider {
    pub fn new() -> Supercollider {

        let mut process = Popen::create(
            &["sclang", "src/scd/start_server.scd", "-u", "13336"],
            PopenConfig { stdout: Redirection::Merge, ..Default::default() }
        ).unwrap();

        // Note: this port is targeted by start_server.scd
        let recv_addr = match SocketAddrV4::from_str("127.0.0.1:13338") {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding incoming osc address"),
        };

        let incoming_socket = UdpSocket::bind(recv_addr).unwrap();

        let scsynth_addr = match SocketAddrV4::from_str("127.0.0.1:13337") {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding scsynth address"),
        };

        let sclang_addr = match SocketAddrV4::from_str("127.0.0.1:13336") {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding sclang address"),
        };

        // Note this sneaky configuration. Mostly needed so that wait_for method does not stay on forever ...
        // Seems to also enable ctrl+c interrupt for some reason.
        incoming_socket.set_read_timeout(Option::Some(Duration::from_secs(30)));

        Supercollider {
            sclang_process: process,
            osc_socket: incoming_socket,
            sclang_out_addr: sclang_addr,
            scsynth_out_addr: scsynth_addr,
        }
    }

    pub fn is_alive(&mut self) -> bool {
        !self.sclang_process.poll().is_some()
    }

    pub fn terminate(&mut self) {
        println!("Exiting sclang...");
        self.sclang_process.terminate();
    }

    pub fn send_to_server(&self, msg: OscMessage) {
        let msg_buf = encoder::encode(&OscPacket::Message(
            msg
        )).unwrap();

        self.osc_socket.send_to(&msg_buf, self.scsynth_out_addr).unwrap();
    }

    pub fn send_to_client(&self, msg: OscMessage) {
        let msg_buf = encoder::encode(&OscPacket::Message(
            msg
        )).unwrap();

        self.osc_socket.send_to(&msg_buf, self.sclang_out_addr).unwrap();
    }

    pub fn wait_for(&self, message_name: &str, args: Vec<OscType>, timeout: Duration) {

        let start_time = Instant::now();

        let mut buf = [0u8; rosc::decoder::MTU];

        println!(">> Waiting for message with name {} and args {:?} ...", message_name, args);

        loop {

            match self.osc_socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    //println!("Received packet with size {} from: {}", size, addr);
                    let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                    match packet {
                        OscPacket::Message(msg) => {
                            //println!("OSC address: {}", msg.addr);
                            //println!("OSC arguments: {:?}", msg.args);

                            if  msg.addr == message_name && args == msg.args {
                                println!(">> Awaited message received! Continuing ...");
                                break;
                            } else {
/*                                println!(
                                    "Name does not match {} != {} or {:?} != {:?}",
                                         msg.addr,
                                         message_name,
                                         msg.args,
                                         args
                                );*/
                            }
                        }
                        OscPacket::Bundle(bundle) => {
                            //println!("OSC Bundle: {:?}", bundle);
                        }
                    }
                }
                Err(e) => {
                    println!(">> Error receiving from socket: {}", e);
                    break;
                }
            }

            let elapsed = start_time.elapsed();

            println!("Elapsed: {:?}, timeout: {:?}", elapsed.clone(), timeout.clone());

            if elapsed > timeout {
                println!(">> Timed out waiting for {}", message_name);
                break;
            }

            std::thread::sleep(Duration::from_millis(10));
        }

    }
}