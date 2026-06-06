use std::fs;
use std::path::Path;

use crate::config;

pub fn read_scd_file(template_name: &str) -> String {

    let full_path = "src/scd/".to_string() + template_name;
    let text = fs::read_to_string(Path::new(&full_path))
        .map_err(|e| format!("Cannot find template script '{}' in source files: {}", full_path, e)).unwrap();

    text
}

pub fn create_nrt_script(
    bpm: f32,
    file_name: &str,
    end_time: f32,
    message_scd_rows: Vec<String>
) -> String {
    let cfg = config::Config::get();

    let mut text = read_scd_file("nrt_record.scd.template");

    let score_row = message_scd_rows.join(",\n");

    text = text.replace("{:bpm}", &format!("{}", bpm));
    text = text.replace("{:file_name}", file_name);
    text = text.replace("{:score_rows}", &score_row);
    text = text.replace("{:end_time}", &format!("{}", end_time));
    text = text.replace("{:out_socket_name}", &cfg.server_osc_socket_name);

    text

}

pub fn create_boot_script() -> Result<String, String> {
    let cfg = config::Config::get();

    let mut text = read_scd_file("start_server.scd.template");
    text = text.replace("{:server_out_port}", &cfg.server_out_port.to_string());
    text = text.replace("{:server_in_port}", &cfg.server_in_port.to_string());
    text = text.replace("{:application_ip}", &cfg.application_ip);
    text = text.replace("{:server_name}", &cfg.server_name);
    text = text.replace("{:out_socket_name}", &cfg.server_osc_socket_name);
    text = text.replace("{:memory_bytes}", &cfg.supercollider_memory_bytes.to_string());

    return Ok(text);
}

// Take synthdef code and wrap it in an nrt score line
pub fn nrt_wrap_synthdef(def_code: &str) -> String {
    // NOTE: Supercollider documentation recommends the writeDefFile method for larger
    // synthDefs. Since we have no control over how large a synthDef any user can create,
    // it is probably best long term to change the method into writing to temp synthDef files.
    format!("[0.0, ['/d_recv', {}]]", def_code)
}
