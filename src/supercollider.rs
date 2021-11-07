use subprocess::{Popen, PopenConfig, Redirection};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use rosc::{OscPacket, OscTime, OscType, OscMessage, encoder};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::{Mutex, Arc};
use std::cell::RefCell;

pub struct NodeManager {
    sc_handle: Arc<Mutex<Supercollider>>,
    current_node_id: RefCell<i32>,
}

impl NodeManager {
    pub fn new(sc_handle: Arc<Mutex<Supercollider>>) -> NodeManager {
        NodeManager {
            sc_handle,
            current_node_id: RefCell::new(100)
        }
    }

    pub fn s_new_timed_gate(&self, synth_name: &str, args: Vec<OscType>, gate_time_sec: f32) {

        let current_id = self.current_node_id.clone().into_inner();
        self.current_node_id.replace(current_id + 1);

        let mut final_args = vec![
            OscType::String(synth_name.to_string()),
            OscType::Int(current_id), // NodeID
            OscType::Int(0), // Group?
            OscType::Int(0), // Group placement?
        ];

        final_args.extend(args);

        self.sc_handle.lock().unwrap().send_to_server(
            OscMessage {
                addr: "/s_new".to_string(),
                args:  final_args
            });

        let handle_clone = self.sc_handle.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs_f32(gate_time_sec));
            handle_clone.lock().unwrap().send_to_server(
                OscMessage {
                    addr: "/n_set".to_string(),
                    args: vec![
                        OscType::Int(current_id), // NodeID
                        OscType::String("gate".to_string()), // Gate is the "note off" signal
                        OscType::Int(0),
                    ]
                });
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

    pub fn wait_for(&self, message_name: &str, args: Vec<OscType>, abort_check: Arc<Mutex<RefCell<bool>>>) {

        let mut buf = [0u8; rosc::decoder::MTU];

        loop {

            // TODO: Does not appear to work when hitting ctrl+c on server boot 
            if abort_check.lock().unwrap().clone().into_inner() == true {
                println!("wait_for cancelled by manual abort");
                break;
            }

            println!("Waiting for message with name {} and args {:?}", message_name, args);
            match self.osc_socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    //println!("Received packet with size {} from: {}", size, addr);
                    let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                    match packet {
                        OscPacket::Message(msg) => {
                            println!("OSC address: {}", msg.addr);
                            //println!("OSC arguments: {:?}", msg.args);

                            if  msg.addr == message_name && args == msg.args {
                                println!("Awaited message received!");
                                break;
                            } else {
                                println!(
                                    "Name does not match {} != {} or {:?} != {:?}",
                                         msg.addr,
                                         message_name,
                                         msg.args,
                                         args
                                );
                            }
                        }
                        OscPacket::Bundle(bundle) => {
                            println!("OSC Bundle: {:?}", bundle);
                        }
                    }
                }
                Err(e) => {
                    println!("Error receiving from socket: {}", e);
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }

    }
}