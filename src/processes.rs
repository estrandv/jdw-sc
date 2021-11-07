use subprocess::{Popen, PopenConfig, Redirection};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use rosc::{OscPacket, OscTime, OscType};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Supercollider {
    sclang_process: Popen,
    sclang_incoming_osc: UdpSocket,
}

impl Supercollider {
    pub fn new() -> Supercollider {

        let mut process = Popen::create(
            &["sclang", "src/scd/start_server.scd", "-u", "13336"],
            PopenConfig { stdout: Redirection::Merge, ..Default::default() }
        ).unwrap();


        // Note: address is from start_server.scd
        let addr = match SocketAddrV4::from_str("127.0.0.1:13338") {
            Ok(addr) => addr,
            Err(_) => panic!("error in osc poll"),
        };

        let socket = UdpSocket::bind(addr).unwrap();

        Supercollider {
            sclang_process: process,
            sclang_incoming_osc: socket
        }
    }

    pub fn is_alive(&mut self) -> bool {
        !self.sclang_process.poll().is_some()
    }

    pub fn terminate(&mut self) {
        println!("Exiting sclang...");
        self.sclang_process.terminate();
    }

    pub fn wait_for(&self, message_name: &str, args: Vec<OscType>) {

        let mut buf = [0u8; rosc::decoder::MTU];

        loop {
            println!("Waiting for message with name {} and args {:?}", message_name, args);
            match self.sclang_incoming_osc.recv_from(&mut buf) {
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

// NOTE: early POC that I just left around for process handling inspo
pub struct ProcessHandler {
    processes: Vec<Popen>
}

impl ProcessHandler {

    pub fn new() -> ProcessHandler {
        ProcessHandler {
            processes: Vec::new()
        }
    }

    pub fn create(&mut self, argv: &[&str]) {
        let mut p = Popen::create(argv, PopenConfig {
            stdout: Redirection::Merge, ..Default::default()
        }).unwrap();

        self.processes.push(p);

    }

    pub fn terminate_all(&mut self) {
        for p in &mut self.processes {
            println!("Killing a process!");
            p.terminate();
        }
    }

    pub fn any_alive(&mut self) -> bool {
        for p in &mut self.processes {
            if p.poll().is_some() {
                return true;
            }
        }
        return false;
    }
}