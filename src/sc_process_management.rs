use crate::config::{
    OUTGOING_PORT, SCLANG_IN_PORT, SC_SERVER_INCOMING_READ_TIMEOUT, SERVER_IN_PORT, SERVER_OUT_PORT,
};
use crate::{config, scd_templating};
use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use jdw_osc_lib::model::TimedOSCPacket;
use log::{debug, info};
use rosc::{encoder, OscBundle, OscMessage, OscPacket, OscTime, OscType};
use std::fs::File;
use std::io::Write;
use std::net::{SocketAddrV4, UdpSocket};
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use std::{fs, thread};
use subprocess::{Popen, PopenConfig, Redirection};

pub struct SCInitData {
    pub client: SCClient,
    pub process: Popen,
}

pub fn init() -> Result<SCInitData, Box<dyn std::error::Error>> {
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

    let process = Popen::create(
        &[
            "sclang",
            "temp/start_server.scd",
            "-u",
            &SCLANG_IN_PORT.to_string(),
        ],
        PopenConfig {
            stdout: Redirection::Merge,
            ..Default::default()
        },
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

    let out_addr = match SocketAddrV4::from_str(&config::get_addr(OUTGOING_PORT)) {
        Ok(addr) => addr,
        Err(_) => panic!("Error binding outgoing traffic address"),
    };

    // Note this sneaky configuration. Mostly needed so that wait_for method does not stay on forever ...
    // Seems to also enable ctrl+c interrupt for some reason.
    incoming_socket
        .set_read_timeout(Option::Some(Duration::from_secs(
            SC_SERVER_INCOMING_READ_TIMEOUT,
        )))
        .unwrap();

    let client = SCClient {
        osc_socket: incoming_socket,
        sclang_out_addr: sclang_addr,
        scsynth_out_addr: scsynth_addr,
        application_out_addr: out_addr,
    };

    Ok(SCInitData { client, process })
}

pub struct SCClient {
    osc_socket: UdpSocket,
    sclang_out_addr: SocketAddrV4,
    scsynth_out_addr: SocketAddrV4,
    application_out_addr: SocketAddrV4,
}

impl SCClient {
    /*
       Note on delay: supercollider execution time can vary by a few milliseconds.
       By providing a delay, we remove this variation via specifying the exact time of execution.
       This is important in precise sequencing but unimportant for direct human input.
    */
    pub fn send_timed_packets_to_scsynth(&self, delay_ms: u64, msgs: Vec<TimedOSCPacket>) {
        for msg in msgs {
            if msg.time == BigDecimal::zero() {
                self.send_to_scsynth_with_delay(msg.packet, delay_ms);
            } else {
                // Tell supercollider to execute the message after a delay
                let time_in_ms = BigDecimal::from_str("1000.00").unwrap() * msg.time.clone();
                let time_integer = time_in_ms.to_u64().unwrap();
                self.send_to_scsynth_with_delay(msg.packet, delay_ms + time_integer);
            }
        }
    }

    pub fn send_to_scsynth_with_delay(&self, msg: OscPacket, delay_ms: u64) {
        // TODO: Trying out some latency adjustments to fix desync issues
        // This is not the optimal way - these operations are highly reliant on context

        let target_time = SystemTime::now() + Duration::from_millis(delay_ms);

        use std::convert::TryFrom;
        let bundle = OscBundle {
            timetag: OscTime::try_from(target_time).unwrap(),
            content: vec![msg],
        };

        let packet = OscPacket::Bundle(bundle);

        // NOTE: Used to just send &msg here
        let msg_buf = encoder::encode(&packet).unwrap();

        self.osc_socket
            .send_to(&msg_buf, self.scsynth_out_addr)
            .unwrap();
    }

    pub fn send_out(&self, msg: OscMessage) {
        let msg_buf = encoder::encode(&OscPacket::Message(msg)).unwrap();
        self.osc_socket
            .send_to(&msg_buf, self.application_out_addr)
            .unwrap();
    }

    pub fn send_to_sclang(&self, msg: OscMessage) {
        let msg_buf = encoder::encode(&OscPacket::Message(msg)).unwrap();

        self.osc_socket
            .send_to(&msg_buf, self.sclang_out_addr)
            .unwrap();
    }

    /*
        Await an OSC message sent from the out_socket used by managed processes.
    */
    pub fn await_internal_response(
        &self,
        message_name: &str,
        args: Vec<OscType>,
        timeout: Duration,
    ) -> Result<(), String> {
        let start_time = Instant::now();

        let mut buf = [0u8; rosc::decoder::MTU];

        info!(
            ">> Waiting for message with name {} and args {:?} ...",
            message_name, args
        );

        loop {
            // TODO: Timeout does not work for recv_from - if no messages arrive at all it will forever-loop
            //  The internal timeout arg only works for when other messages arrive consistently until timeout
            //self.osc_socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap()
            match self.osc_socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();

                    match packet {
                        OscPacket::Message(msg) => {
                            if msg.addr == message_name && args == msg.args {
                                info!(">> Awaited message received! Continuing ...");
                                return Ok(());
                            } else {
                                debug!(
                                    "Received message not the waited for one, continuing wait..."
                                );
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

            debug!(
                "Elapsed: {:?}, timeout: {:?}",
                elapsed.clone(),
                timeout.clone()
            );

            if elapsed > timeout {
                return Err(format!(">> Timed out waiting for {}", message_name));
            }

            thread::sleep(Duration::from_millis(10));
        }
    }
}
