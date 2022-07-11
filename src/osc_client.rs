extern crate rosc;

use rosc::{OscPacket, decoder};
use std::env;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;

// NOTE: Naming is perhaps suboptimal. This mainly concerns receiving external OSC messages.
// Different OSC handling is also used in the supercollider.rs functions.

// TODO: Use https://github.com/klingtnet/rosc/blob/master/examples/receiver.rs
// Prob as an extremely simple struct that polls for OscPacket which can then be differentiated
// in the main loop.

pub struct OSCPoller {
    socket: UdpSocket,
    buf: [u8; 1536]
}

impl OSCPoller {

    pub fn new() -> OSCPoller {
        // TODO: config port
        let addr = match SocketAddrV4::from_str("127.0.0.1:13331") {
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

