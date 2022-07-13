use std::fs::DirEntry;
use std::{io, fs};
use std::fmt::format;
use std::path::Path;
use log::{debug, info};
use crate::config::{APPLICATION_IP, SERVER_IN_PORT, SERVER_NAME, SERVER_OSC_SOCKET_NAME, SERVER_OUT_PORT, SUPERCOLLIDER_MEMORY_BYTES};

pub fn create_boot_script() -> Result<String, String> {
    let mut text = fs::read_to_string(Path::new("src/scd/start_server.scd"))
        .map_err(|e| format!("{}", e))?;
    text = text.replace("{:server_out_port}", &SERVER_OUT_PORT.to_string());
    text = text.replace("{:server_in_port}", &SERVER_IN_PORT.to_string());
    text = text.replace("{:application_ip}", APPLICATION_IP);
    text = text.replace("{:server_name}", SERVER_NAME);
    text = text.replace("{:out_socket_name}", SERVER_OSC_SOCKET_NAME);
    text = text.replace("{:memory_bytes}", &SUPERCOLLIDER_MEMORY_BYTES.to_string());

    return Ok(text);
}

// TODO: Return result, clarify operation naming
pub fn read_all_synths(operation: &str) -> Vec<String> {
    let path = Path::new("src/scd/synths");

    let mut result: Vec<String> = Vec::new();

    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        let raw_text = fs::read_to_string(path.clone()).unwrap();

        let synth_name = path.file_stem().unwrap().to_str().unwrap().to_string();


        let mut text = raw_text.replace("{:operation}", operation);
        text = text.replace("{:synth_name}", &synth_name);

        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        debug!("Reading: {}", file_name.clone());

        // Add a postln to the end so that we see a confirmation message in console.
        let with_load_msg = text + &format!("\n\"{} loaded.\".postln;", file_name);

        result.push(with_load_msg);

    }

    result

}
