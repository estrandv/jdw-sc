use std::{fs, thread};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Empty, Write};
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
use crate::samples::SamplePackCollection;

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

pub struct SCProcessManager {
    sclang_process: Popen,
    osc_socket: UdpSocket,
    sclang_out_addr: SocketAddrV4,
    scsynth_out_addr: SocketAddrV4,
}

impl SCProcessManager {
    pub fn new() -> Result<SCProcessManager, Box<dyn std::error::Error>> {

        // TODO: General temp folder management should be its own little util
        // TODO: ... and use home folder instead

        info!("Generating boot script");

        let templated = scd_templating::create_boot_script()?;

        info!("Creating temp dir");

        let temp_dir = Path::new("temp");
        if !temp_dir.exists() {
            fs::create_dir(Path::new("temp"))?;
        }

        info!("Creating server boot script temp file");

        let mut file = File::create("temp/start_server.scd")?;
        file.write_all(templated.as_bytes())?;

        info!("Starting supercollider with generated boot script");

        let mut process = Popen::create(
            &["sclang", "temp/start_server.scd", "-u", &SCLANG_IN_PORT.to_string()],
            PopenConfig { stdout: Redirection::Merge, ..Default::default() }
        )?;

        // Note: this port is targeted by start_server.scd.template
        // Note: Technically the second UDP in socket managed by the application,
        // the other being the public in-port used to send messages to jdw-sc
        let recv_addr = match SocketAddrV4::from_str(&config::get_addr(SERVER_OUT_PORT)) {
            Ok(addr) => addr,
            Err(_) => panic!("Error binding incoming osc address"),
        };

        let incoming_socket = UdpSocket::bind(recv_addr)?;

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

        Ok(SCProcessManager {
            sclang_process: process,
            osc_socket: incoming_socket,
            sclang_out_addr: sclang_addr,
            scsynth_out_addr: scsynth_addr,
        })
    }

    pub fn is_alive(&mut self) -> bool {
        !self.sclang_process.poll().is_some()
    }

    pub fn terminate(&mut self) {
        info!("Exiting sclang...");
        self.sclang_process.terminate().unwrap();
    }

    pub fn send_timed_packets(handle: Arc<Mutex<SCProcessManager>>, msgs: Vec<TimedOSCPacket>) {

        for msg in msgs {
            if msg.time == BigDecimal::zero() {
                handle.lock().unwrap().send_with_delay(msg.packet, config::LATENCY_MS);
            } else {
                // Tell supercollider to execute the message after a delay
                let time_in_ms = BigDecimal::from_str("1000.00").unwrap() * msg.time.clone();
                let time_integer = time_in_ms.to_u64().unwrap();
                handle.lock().unwrap().send_with_delay(msg.packet, config::LATENCY_MS + time_integer);
            }
        }
    }

    fn send_with_delay(&self, msg: OscPacket, delay_ms: u64) {
        // TODO: Trying out some latency adjustments to fix desync issues
        // This is not the optimal way - these operations are highly reliant on context

        let now = SystemTime::now() + Duration::from_millis(delay_ms);

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

    pub fn await_response(&self, message_name: &str, args: Vec<OscType>, timeout: Duration) -> Result<(), String> {

        let start_time = Instant::now();

        let mut buf = [0u8; rosc::decoder::MTU];

        info!(">> Waiting for message with name {} and args {:?} ...", message_name, args);

        loop {

            // TODO: Timeout does not work for recv_from - if no messages arrive at all it will forever-loop
            //  The internal timeout arg only works for when other messages arrive consistently until timeout
            //self.osc_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap()
            match self.osc_socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();

                    match packet {
                        OscPacket::Message(msg) => {

                            if  msg.addr == message_name && args == msg.args {
                                info!(">> Awaited message received! Continuing ...");
                                return Ok(());
                            } else {
                                debug!("Received message not the waited for one, continuing wait...");
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(format!(">> Error receiving from socket: {}", e));
                }
            }

            let elapsed = start_time.elapsed();

            debug!("Elapsed: {:?}, timeout: {:?}", elapsed.clone(), timeout.clone());

            if elapsed > timeout {
                return Err(format!(">> Timed out waiting for {}", message_name));
            }

            thread::sleep(Duration::from_millis(10));
        }

    }
}