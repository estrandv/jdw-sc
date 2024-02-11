use std::{fs, thread};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::net::{SocketAddrV4, UdpSocket};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};
use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use jdw_osc_lib::model::TimedOSCPacket;

use log::{debug, info, warn};
use regex::{Error, Regex};
use rosc::{encoder, OscBundle, OscMessage, OscPacket, OscTime, OscType};
use subprocess::{Popen, PopenConfig, Redirection};

use crate::{config, scd_templating};
use crate::config::{SC_SERVER_INCOMING_READ_TIMEOUT, SCLANG_IN_PORT, SERVER_IN_PORT, SERVER_OUT_PORT};
use crate::samples::SampleDict;

fn get_arg(args: Vec<OscType>, arg_name: &str) -> Option<OscType> {
        let pos = args.iter().position(|arg| arg.clone() == OscType::String(arg_name.to_string()));

        match pos {
            Some(index) => {
                let value_pos = index + 1;

                match args.get(value_pos) {
                    Some(val) => Some(val.clone()),
                    None => None
                }

            },
            None => None
        }

}

pub struct Supercollider {
    sclang_process: Popen,
    osc_socket: UdpSocket,
    sclang_out_addr: SocketAddrV4,
    scsynth_out_addr: SocketAddrV4,
}

impl Supercollider {
    // TODO: Do result. Shut down application on any bind failures.
    pub fn new() -> Supercollider {

        // TODO: Error handling
        // TODO: General temp folder management should be its own little util
        let templated = scd_templating::create_boot_script().unwrap();
        let temp_dir = Path::new("temp");
        if !temp_dir.exists() {
            fs::create_dir(Path::new("temp")).unwrap();
        }
        let mut file = File::create("temp/start_server.scd").unwrap();
        file.write_all(templated.as_bytes()).unwrap();

        let mut process = Popen::create(
            &["sclang", "temp/start_server.scd", "-u", &SCLANG_IN_PORT.to_string()],
            PopenConfig { stdout: Redirection::Merge, ..Default::default() }
        ).unwrap();

        // Note: this port is targeted by start_server.scd
        // Note: Technically the second UDP in socket managed by the application,
        // the other being the public in-port used to send messages to jdw-sc
        let recv_addr = match SocketAddrV4::from_str(&config::get_addr(SERVER_OUT_PORT)) {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding incoming osc address"),
        };

        let incoming_socket = UdpSocket::bind(recv_addr).unwrap();

        let scsynth_addr = match SocketAddrV4::from_str(&config::get_addr(SERVER_IN_PORT)) {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding scsynth address"),
        };

        let sclang_addr = match SocketAddrV4::from_str(&config::get_addr(SCLANG_IN_PORT)) {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding sclang address"),
        };

        // Note this sneaky configuration. Mostly needed so that wait_for method does not stay on forever ...
        // Seems to also enable ctrl+c interrupt for some reason.
        incoming_socket.set_read_timeout(Option::Some(Duration::from_secs(SC_SERVER_INCOMING_READ_TIMEOUT)));

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
        info!("Exiting sclang...");
        self.sclang_process.terminate();
    }

    pub fn send_timed(handle: Arc<Mutex<Supercollider>>, msgs: Vec<TimedOSCPacket>) {

        for msg in msgs {
            if msg.time == BigDecimal::zero() {
                // TODO: No need for threading. Internal timing can be added to latency just as well
                //  if we add latency on this level and send it as a parameter
                handle.lock().unwrap().send_to_server(msg.packet);
            } else {
                let handle_clone = handle.clone();
                thread::spawn(move || {
                    let time_in_microsec = BigDecimal::from_str("1000000.00").unwrap() * msg.time.clone();
                    let time_integer = time_in_microsec.to_u64().unwrap();
                    thread::sleep(Duration::from_micros(time_integer));
                    handle_clone.lock().unwrap().send_to_server(msg.packet);
                    // TODO: Bit of a lifetime mess here - we actually want to call remove_running or even note_off
                    //  at this point but moving mut self is an issue...
                    // If running notes member is converted to refcell we can cheat the &mut self of remove_running
                    // Also note that we should technically remove it after release time rather than immediately on gate
                    // ... which kinda also means that "rel" arg should be elevated to a message-level arg... but
                    // that is shaky territory.
                });
            }
        }
    }

    pub fn send_to_server(&self, msg: OscPacket) {

        // TODO: Trying out some latency adjustments to fix desync issues
        // This is not the optimal way - these operations are highly reliant on context

        let now = SystemTime::now() + Duration::from_millis(config::LATENCY_MS);

        use std::convert::TryFrom;
        let bundle = OscBundle {
            timetag: OscTime::try_from(now).unwrap(),
            content: vec![msg]
        };

        let packet = OscPacket::Bundle(bundle);

        // NOTE: Used to just send &msg here
        let msg_buf = encoder::encode(&packet).unwrap();

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

        info!(">> Waiting for message with name {} and args {:?} ...", message_name, args);

        loop {

            match self.osc_socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();

                    match packet {
                        OscPacket::Message(msg) => {

                            if  msg.addr == message_name && args == msg.args {
                                info!(">> Awaited message received! Continuing ...");
                                break;
                            } else {
                                debug!("Received message not the waited for one, continuing wait...");
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    warn!(">> Error receiving from socket: {}", e);
                    break;
                }
            }

            let elapsed = start_time.elapsed();

            debug!("Elapsed: {:?}, timeout: {:?}", elapsed.clone(), timeout.clone());

            if elapsed > timeout {
                warn!(">> Timed out waiting for {}", message_name);
                break;
            }

            std::thread::sleep(Duration::from_millis(10));
        }

    }
}