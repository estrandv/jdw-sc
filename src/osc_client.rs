extern crate rosc;

use rosc::{OscPacket, decoder};
use std::env;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;
use crate::config;
use crate::config::{APPLICATION_IN_PORT, APPLICATION_IP};

// NOTE: Naming is perhaps suboptimal. This mainly concerns receiving external OSC messages.
// Different OSC handling is also used in the supercollider.rs functions.

pub struct OSCPoller {
    socket: UdpSocket,
    buf: [u8; 1536]
}

impl OSCPoller {

    pub fn new() -> OSCPoller {
        let addr = match SocketAddrV4::from_str(&config::get_addr(APPLICATION_IN_PORT)) {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let sock = UdpSocket::bind(addr).unwrap();
        let buf = [0u8; rosc::decoder::MTU];

        OSCPoller {
            socket: sock,
            buf
        }

    }

    pub fn poll(&mut self) -> Result<OscPacket, String> {
        return match self.socket.recv_from(&mut self.buf) {
            Ok((size, addr)) => {
                let (_, packet) = rosc::decoder::decode_udp(&self.buf[..size]).unwrap();
                Ok(packet)
            }
            Err(e) => {Err("Error receiving from osc socket".to_string())}
        };
    }
}

