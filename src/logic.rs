use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use tokio::sync::watch;

use crate::config::Config;
use crate::controller::DualSense;
use crate::telemetry::UdpListener;
use crate::effects::ControllerLogic;

#[derive(Clone, Default)]
pub struct EngineState {
    pub is_connected: bool,
    pub rpm: f32,
    pub max_rpm: f32,
    pub speed_kmh: f32,
}

pub struct LogicLoop {
    config: Config,
    config_rx: Receiver<Config>,
    state_tx: watch::Sender<EngineState>,
    listener: UdpListener,
    controller: DualSense,
}

impl LogicLoop {
    pub fn new(config: Config, config_rx: Receiver<Config>, state_tx: watch::Sender<EngineState>) -> Result<Self, String> {
        let listener = UdpListener::new(&config.udp_host, config.udp_port, config.udp_timeout)
            .map_err(|e| format!("Failed to start UDP listener: {}", e))?;
        
        let mut controller = DualSense::new()?;
        controller.try_connect();

        Ok(Self {
            config,
            config_rx,
            state_tx,
            listener,
            controller,
        })
    }

    pub fn run(mut self) {
        let mut ctrl_logic = ControllerLogic::new();
        let mut last_state = EngineState::default();

        loop {
            // Hot reload config if available
            if let Ok(new_cfg) = self.config_rx.try_recv() {
                println!("Config reloaded in logic thread");
                self.config = new_cfg;
            }

            // Read telemetry
            let pkt = self.listener.recv_latest();
            
            // Reconnect logic
            let was_connected = self.controller.is_connected();
            if !was_connected {
                self.controller.try_connect();
            }
            let is_connected = self.controller.is_connected();

            if !is_connected {
                last_state.is_connected = false;
                last_state.rpm = 0.0;
                last_state.speed_kmh = 0.0;
                let _ = self.state_tx.send_if_modified(|s| {
                    let changed = s.is_connected != last_state.is_connected || s.rpm != last_state.rpm;
                    *s = last_state.clone();
                    changed
                });
                
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }

            let now = Instant::now();
            let (left_mode, right_mode) = if let Some(t) = pkt {
                last_state.is_connected = true;
                last_state.rpm = t.rpm;
                last_state.max_rpm = if t.max_rpm > 0.0 { t.max_rpm } else { 1.0 };
                last_state.speed_kmh = t.speed * 3.6; // convert m/s to km/h

                ctrl_logic.update(t, &self.config, now)
            } else {
                last_state.is_connected = true;
                last_state.rpm = 0.0;
                last_state.speed_kmh = 0.0;

                (crate::effects::off(), crate::effects::off())
            };

            let _ = self.state_tx.send_if_modified(|s| {
                *s = last_state.clone();
                true // Always update UI roughly at loop rate when connected, or we can debounce
            });

            // Send report to DualSense
            let _ = self.controller.update(&left_mode, &right_mode);
        }
    }
}
