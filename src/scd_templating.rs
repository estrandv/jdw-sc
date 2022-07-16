use std::fs::DirEntry;
use std::{io, fs};
use std::fmt::format;
use std::path::Path;
use log::{debug, info};
use crate::config::{APPLICATION_IP, SERVER_IN_PORT, SERVER_NAME, SERVER_OSC_SOCKET_NAME, SERVER_OUT_PORT, SUPERCOLLIDER_MEMORY_BYTES};

pub fn create_nrt_script(
    bpm: f32,
    file_name: &str,
    end_time: f32,
    message_scd_rows: Vec<String>
) -> Result<String, String> {

    let mut text = fs::read_to_string(Path::new("src/scd/nrt_record.scd"))
        .map_err(|e| format!("{}", e))?;

    /*
        Row format:
        [0.0, [msg...]],
        [0.0, Buffer.new(...)],

        Buffer example:
        (Buffer.new(server, 44100 * 8.0, 2, bufnum: 2)).allocReadMsg(File.getcwd +/+ "sample_packs/DR660/DR606 808 Closed Hat 2.wav")
        - this should be accounted for in to_nrt_scd_row()

        TODO:
        - Runningnote to scd rows
            - address, id and args
            - basically full osc - do we keep address? 

     */
    let score_row = message_scd_rows.join(",\n");

    // TODO: Problem. Managed messages arrive without bpm with times in seconds.
    //  Maybe they shouldn't? Conversion is not expensive.
    text = text.replace("{:bpm}", &format!("{}", bpm));
    text = text.replace("{:file_name}", file_name);
    text = text.replace("{:score_rows}", &score_row);
    text = text.replace("{:end_time}", &format!("{}", end_time));

    Ok(text)

}

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
        // TODO: Cute idea, but breaks nrt record among other things. Fetch elsewhere.
        //let with_load_msg = text + &format!("\n\"{} loaded.\".postln;", file_name);

        result.push(text);

    }

    result

}

// Take synthdef code and wrap it in an nrt score line
pub fn nrt_wrap_synthdef(def_code: &str) -> String {
    format!("[0.0, ['/d_recv', {}]]", def_code)
}
