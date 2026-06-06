#![feature(result_flattening)]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let config_path = args
        .iter()
        .skip(1)
        .skip_while(|a| a.starts_with('-'))
        .next()
        .map(|s| s.as_str())
        .unwrap_or("config.toml");

    jdw_sc::run(config_path, quiet);
}
