use log::LevelFilter;
use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;
use toml::Value as TomlValue;

static CONFIG: OnceLock<Config> = OnceLock::new();
static APP_NAME: &str = "sc";

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

fn central_config_path() -> String {
    if let Ok(path) = std::env::var("JDW_CONFIG") {
        if Path::new(&path).exists() {
            return path;
        }
    }
    let home = std::env::var("HOME").ok();
    if let Some(home) = home {
        let xdg = Path::new(&home).join(".config").join("jdw.toml");
        if xdg.exists() {
            return xdg.to_string_lossy().to_string();
        }
    }
    eprintln!("Error: Central config not found at ~/.config/jdw.toml");
    eprintln!("       Set $JDW_CONFIG to a custom path, or create the file.");
    std::process::exit(1);
}

fn load_central_section() -> Option<TomlValue> {
    let path = central_config_path();
    let contents = std::fs::read_to_string(&path).ok()?;
    let root: TomlValue = contents.parse().ok()?;
    root.get(APP_NAME).cloned()
}

fn merge_str(base: &mut String, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_str()) {
        *base = v.to_string();
    }
}

fn merge_i32(base: &mut i32, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as i32;
    }
}

fn merge_u64(base: &mut u64, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as u64;
    }
}

fn merge_usize(base: &mut usize, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_integer()) {
        *base = v as usize;
    }
}

fn merge_f64(base: &mut f64, overlay: &TomlValue, key: &str) {
    if let Some(v) = overlay.get(key).and_then(|v| v.as_float()) {
        *base = v;
    }
}

fn merge_config(base: &mut Config, overlay: &TomlValue) {
    merge_str(&mut base.application_ip, overlay, "application_ip");
    merge_str(&mut base.server_osc_socket_name, overlay, "server_osc_socket_name");
    merge_str(&mut base.server_name, overlay, "server_name");
    merge_u64(&mut base.sc_server_incoming_read_timeout, overlay, "sc_server_incoming_read_timeout");
    merge_i32(&mut base.server_out_port, overlay, "server_out_port");
    merge_i32(&mut base.sclang_in_port, overlay, "sclang_in_port");
    merge_i32(&mut base.server_in_port, overlay, "server_in_port");
    merge_i32(&mut base.outgoing_port, overlay, "outgoing_port");
    merge_i32(&mut base.application_in_port, overlay, "application_in_port");
    merge_i32(&mut base.supercollider_memory_bytes, overlay, "supercollider_memory_bytes");
    merge_str(&mut base.log_level, overlay, "log_level");
    merge_u64(&mut base.init_wait_timeout_secs, overlay, "init_wait_timeout_secs");
    merge_str(&mut base.sample_pack_dir, overlay, "sample_pack_dir");

    merge_str(&mut base.sclang_binary, overlay, "sclang_binary");
    merge_u64(&mut base.poll_sleep_ms, overlay, "poll_sleep_ms");
    merge_i32(&mut base.default_bpm, overlay, "default_bpm");
    merge_usize(&mut base.buffer_size, overlay, "buffer_size");
    merge_u64(&mut base.nrt_done_timeout_secs, overlay, "nrt_done_timeout_secs");
    merge_i32(&mut base.first_node_id, overlay, "first_node_id");
    merge_f64(&mut base.sample_buffer_frames, overlay, "sample_buffer_frames");
    merge_i32(&mut base.sample_channels, overlay, "sample_channels");
    merge_i32(&mut base.group_id, overlay, "group_id");
    merge_i32(&mut base.group_placement, overlay, "group_placement");
}

pub fn load(config_path: &str) -> Config {
    let mut cfg = Config::default();

    if let Some(central) = load_central_section() {
        merge_config(&mut cfg, &central);
    }

    if !config_path.is_empty() {
        if let Ok(contents) = std::fs::read_to_string(config_path) {
            if let Ok(local) = toml::from_str::<TomlValue>(&contents) {
                merge_config(&mut cfg, &local);
            }
        } else {
            eprintln!("Warning: Config file '{}' not found. Using defaults.", config_path);
        }
    }

    cfg
}

pub fn init(path: &str) {
    let config = load(path);
    CONFIG.set(config).ok();
}

pub fn get_addr(port: i32) -> String {
    format!("{}:{}", Config::get().application_ip, port)
}
