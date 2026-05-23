use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // --- UDP ---
    pub udp_host: String,
    pub udp_port: u16,
    pub udp_timeout: f32,

    // --- Shared pedal config ---
    pub pedal_value_max: u8,
    pub wall_zones: u8,

    // =============================================================
    // L2 — Brake pedal
    // =============================================================
    pub enable_brake_resistance: bool,
    pub brake_deadzone: u8,
    pub brake_baseline_force: u8,
    pub brake_max_force: u8,
    pub brake_curve: f32,
    pub brake_wall_engage_at: u8,
    pub brake_wall_release_at: u8,
    pub enable_brake_static_wall: bool,
    pub brake_static_wall_at: u8,
    pub brake_static_wall_force: u8,

    pub enable_handbrake_bonus: bool,
    pub handbrake_bonus: u8,

    pub enable_abs: bool,
    pub abs_brake_threshold: u8,
    pub abs_min_speed_kmh: f32,
    pub abs_slip_ratio_threshold: f32,
    pub abs_combined_slip_threshold: f32,
    pub abs_freq: u8,
    pub abs_amp: u8,

    // =============================================================
    // R2 — Gas pedal
    // =============================================================
    pub enable_throttle_resistance: bool,
    pub accel_deadzone: u8,
    pub throttle_baseline_force: u8,
    pub throttle_max_force: u8,
    pub throttle_curve: f32,
    pub throttle_wall_engage_at: u8,
    pub throttle_wall_release_at: u8,

    pub enable_rev_limiter: bool,
    pub rev_limit_ratio: f32,
    pub rev_limit_freq: u8,
    pub rev_limit_amp: u8,
    pub rev_limit_hold_ms: f32,

    pub enable_wheelspin_buzz: bool,
    pub wheelspin_amp: u8,

    pub enable_gear_shift: bool,
    pub enable_gear_shift_brake: bool,
    pub gear_shift_freq: u8,
    pub gear_shift_amp: u8,
    pub gear_shift_duration_ms: f32,

    // =============================================================
    // System
    // =============================================================
    pub enable_startup_pulse: bool,
    pub startup_pulse_force: u8,

    pub enable_reconnect: bool,
    pub reconnect_interval_s: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            udp_host: "127.0.0.1".to_string(),
            udp_port: 5300,
            udp_timeout: 0.5,

            pedal_value_max: 255,
            wall_zones: 2,

            enable_brake_resistance: true,
            brake_deadzone: 50,
            brake_baseline_force: 20,
            brake_max_force: 80,
            brake_curve: 5.0,
            brake_wall_engage_at: 250,
            brake_wall_release_at: 200,
            enable_brake_static_wall: false,
            brake_static_wall_at: 128,
            brake_static_wall_force: 255,

            enable_handbrake_bonus: true,
            handbrake_bonus: 60,

            enable_abs: true,
            abs_brake_threshold: 80,
            abs_min_speed_kmh: 15.0,
            abs_slip_ratio_threshold: 1.0,
            abs_combined_slip_threshold: 1.0,
            abs_freq: 10,
            abs_amp: 20,

            enable_throttle_resistance: true,
            accel_deadzone: 50,
            throttle_baseline_force: 0,
            throttle_max_force: 8,
            throttle_curve: 5.0,
            throttle_wall_engage_at: 250,
            throttle_wall_release_at: 200,

            enable_rev_limiter: true,
            rev_limit_ratio: 0.93,
            rev_limit_freq: 20,
            rev_limit_amp: 10,
            rev_limit_hold_ms: 120.0,

            enable_wheelspin_buzz: true,
            wheelspin_amp: 3,

            enable_gear_shift: true,
            enable_gear_shift_brake: true,
            gear_shift_freq: 20,
            gear_shift_amp: 255,
            gear_shift_duration_ms: 100.0,

            enable_startup_pulse: true,
            startup_pulse_force: 150,

            enable_reconnect: false,
            reconnect_interval_s: 5.0,
        }
    }
}

impl Config {
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                } else {
                    eprintln!("Warning: Failed to parse config.toml, using defaults.");
                }
            } else {
                eprintln!("Warning: Failed to read config.toml, using defaults.");
            }
        }

        let default_config = Config::default();
        if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
            let _ = fs::write(path, toml_str);
        }
        default_config
    }
}
