use log::{LevelFilter};

/*
    Central place for application configuration until we decide on a non-hardcode method
 */

pub const LOG_LEVEL: LevelFilter = LevelFilter::Debug;
pub const APPLICATION_IP: &str = "127.0.0.1";
pub const SERVER_OSC_SOCKET_NAME: &str = "o";
pub const SERVER_NAME: &str = "s";

pub const SC_SERVER_INCOMING_READ_TIMEOUT: u64 = 30;

// Portconfig note: the server ports need templating in start_server.scd to be changeable
pub const SERVER_OUT_PORT: i32 = 13338;
pub const SCLANG_IN_PORT: i32 = 13336;
pub const SERVER_IN_PORT: i32 = 13337;
pub const APPLICATION_IN_PORT: i32 = 13331;

pub const SUPERCOLLIDER_MEMORY_BYTES: i32 = 2000000; // 2GB

// Supercollider has a small "processing time" that can become noticeable during sequencing
//  if variations grow too large in an otherwise steady beat.
// The latency value tells the server to execute after a small delay, ensuring a more even execution time.
// NOTE: Not confirmed working. It works like this internally (see: PBind) but there is no documentation
//  on its usage for direct OSC communication (i.e. does it use the extra time for pre-processing or simply
//  shift the delay to a later time?).
// If this value is set too low, messages will appear in the log stating "late:" and the amount of overshoot.
// The value below was set after some local testing.
pub const LATENCY_MS: u64 = 70;

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", APPLICATION_IP, port)
}