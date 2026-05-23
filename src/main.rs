mod config;
mod controller;
mod effects;
mod logic;
mod telemetry;
mod trigger_builders;
mod utils;

use std::sync::mpsc;
use tokio::sync::watch;

fn main() {
    let mut cfg = config::Config::load_or_default("config.toml");

    // Support E2E mock testing
    if let Ok(port_str) = std::env::var("MOCK_TELEMETRY_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            cfg.udp_port = port;
            println!("MOCK_TELEMETRY_PORT set to {}", port);
        }
    }

    println!("Loaded config: {:#?}", cfg);

    let (_config_tx, config_rx) = mpsc::channel();
    let (state_tx, _state_rx) = watch::channel(logic::EngineState::default());

    let logic_loop =
        logic::LogicLoop::new(cfg, config_rx, state_tx).expect("Failed to initialize LogicLoop");

    println!("Starting core engine logic loop...");
    logic_loop.run();
}
