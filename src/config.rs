use log::LevelFilter;
use serde::Deserialize;
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct Config {
    pub application_ip: String,
    pub server_osc_socket_name: String,
    pub server_name: String,
    pub sc_server_incoming_read_timeout: u64,
    pub server_out_port: i32,
    pub sclang_in_port: i32,
    pub server_in_port: i32,
    pub outgoing_port: i32,
    pub application_in_port: i32,
    pub supercollider_memory_bytes: i32,
    pub log_level: String,
    pub init_wait_timeout_secs: u64,
    pub sample_pack_dir: String,
    pub temp_dir: String,
    pub sclang_binary: String,
    pub poll_sleep_ms: u64,
    pub default_bpm: i32,
    pub buffer_size: usize,
    pub nrt_done_timeout_secs: u64,
    pub first_node_id: i32,
    pub sample_buffer_frames: f64,
    pub sample_channels: i32,
    pub group_id: i32,
    pub group_placement: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            application_ip: "127.0.0.1".to_string(),
            server_osc_socket_name: "o".to_string(),
            server_name: "s".to_string(),
            sc_server_incoming_read_timeout: 30,
            server_out_port: 13338,
            sclang_in_port: 13336,
            server_in_port: 13337,
            outgoing_port: 13339,
            application_in_port: 13331,
            supercollider_memory_bytes: 2000000,
            log_level: "debug".to_string(),
            init_wait_timeout_secs: 10,
            sample_pack_dir: "~/sample_packs".to_string(),
            temp_dir: "temp".to_string(),
            sclang_binary: "sclang".to_string(),
            poll_sleep_ms: 10,
            default_bpm: 120,
            buffer_size: 333072,
            nrt_done_timeout_secs: 10,
            first_node_id: 100,
            sample_buffer_frames: 44100.0 * 8.0,
            sample_channels: 2,
            group_id: 0,
            group_placement: 0,
        }
    }
}

impl Config {
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized")
    }

    pub fn log_level_filter(&self) -> LevelFilter {
        match self.log_level.to_lowercase().as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Debug,
        }
    }
}

pub fn load(path: &str) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| toml::from_str(&contents).ok())
        .unwrap_or_else(|| {
            eprintln!("No config at '{}', using defaults", path);
            Config::default()
        })
}

pub fn init(path: &str) {
    let config = load(path);
    CONFIG.set(config).ok();
}

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", Config::get().application_ip, port)
}
