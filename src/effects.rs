use std::time::Instant;
use crate::telemetry::ForzaTelemetry;
use crate::config::Config;
use crate::controller::TriggerMode;
use crate::utils::*;

pub fn off() -> TriggerMode { TriggerMode::Off }

pub fn rigid(force: u8) -> TriggerMode { TriggerMode::Rigid(force) }

pub fn vibration(freq: u8, amp: u8) -> TriggerMode { TriggerMode::Pulse(freq, amp) }

pub fn vibration_wall(amp: u8, freq: u8, wall_zones: u8) -> TriggerMode {
    let a = amp.clamp(1, 8);
    let w = wall_zones.clamp(1, 9) as usize;
    let mut zones = [a; 10];
    for i in (10 - w)..10 {
        zones[i] = 8;
    }
    
    let mut active = 0u16;
    let mut strength = 0u32;
    for (i, &s) in zones.iter().enumerate() {
        active |= 1 << i;
        strength |= ((s - 1) as u32) << (3 * i);
    }
    TriggerMode::PulseAB(active, strength, freq)
}

pub fn build_vibrating_resistance(base_force: u8, freq: u8, amp: u8, in_wall: bool, wall_zones: u8) -> TriggerMode {
    let base_strength = amp_to_strength(base_force);
    let buzz_strength = amp_to_strength(amp);
    let combined_strength = base_strength.max(buzz_strength).clamp(1, 8);
    
    let mut zones = [combined_strength; 10];
    if in_wall {
        let w = wall_zones.clamp(1, 9) as usize;
        for i in (10 - w)..10 {
            zones[i] = 8;
        }
    }
    
    let mut active = 0u16;
    let mut strength = 0u32;
    for (i, &s) in zones.iter().enumerate() {
        if s > 0 {
            active |= 1 << i;
            strength |= ((s - 1) as u32) << (3 * i);
        }
    }
    TriggerMode::PulseAB(active, strength, freq)
}

pub fn feedback(zones: &[u8; 10]) -> TriggerMode {
    let mut active = 0u16;
    let mut force = 0u32;
    for (i, &s) in zones.iter().enumerate() {
        let s = s.clamp(0, 8);
        if s > 0 {
            active |= 1 << i;
            force |= ((s - 1) as u32) << (3 * i);
        }
    }
    TriggerMode::Feedback(active, force)
}

pub fn build_wall(wall_zones: u8) -> TriggerMode {
    let w = wall_zones.clamp(1, 9) as usize;
    let mut zones = [0u8; 10];
    for i in (10 - w)..10 {
        zones[i] = 8;
    }
    feedback(&zones)
}

pub fn build_brake_walls(static_at: u8, force_byte: u8, wall_zones: u8) -> TriggerMode {
    let strength = amp_to_strength(force_byte);
    let start = (static_at as usize * 10 / 256).min(9);
    let mut zones = [0u8; 10];
    for i in start..10 {
        zones[i] = strength;
    }
    let w = wall_zones.clamp(1, 9) as usize;
    for i in (10 - w)..10 {
        zones[i] = 8;
    }
    feedback(&zones)
}

pub struct ControllerLogic {
    prev_gear: u8,
    shift_until: Option<Instant>,
    rev_until: Option<Instant>,
    abs_until: Option<Instant>,
    wheelspin_until: Option<Instant>,
    l2_in_wall: bool,
    r2_in_wall: bool,
}

impl ControllerLogic {
    pub fn new() -> Self {
        Self {
            prev_gear: 0,
            shift_until: None,
            rev_until: None,
            abs_until: None,
            wheelspin_until: None,
            l2_in_wall: false,
            r2_in_wall: false,
        }
    }

    pub fn update(&mut self, t: &ForzaTelemetry, s: &Config, now: Instant) -> (TriggerMode, TriggerMode) {
        if t.is_running == 0 {
            return (off(), off());
        }

        if s.enable_gear_shift || s.enable_gear_shift_brake {
            if self.prev_gear != 0 && t.gear != self.prev_gear {
                self.shift_until = Some(now + std::time::Duration::from_secs_f32(s.gear_shift_duration_ms / 1000.0));
            }
            self.prev_gear = t.gear;
        }

        (self.l2(t, s, now), self.r2(t, s, now))
    }

    fn shift_burst(&self, s: &Config, now: Instant, pedal: u8, wall_engage_at: u8) -> Option<TriggerMode> {
        if let Some(until) = self.shift_until {
            if now < until {
                if pedal as u16 >= (wall_engage_at as u16 + 255) / 2 {
                    return Some(vibration_wall(amp_to_strength(s.gear_shift_amp), s.gear_shift_freq, s.wall_zones));
                }
                return Some(vibration(s.gear_shift_freq, s.gear_shift_amp));
            }
        }
        None
    }

    fn l2(&mut self, t: &ForzaTelemetry, s: &Config, now: Instant) -> TriggerMode {
        let brake = t.brake;

        if s.enable_gear_shift_brake {
            if let Some(burst) = self.shift_burst(s, now, brake, s.brake_wall_engage_at) {
                return burst;
            }
        }

        let handbrake = s.enable_handbrake_bonus && t.handbrake > 0;
        let mut base_force = 0;
        if s.enable_brake_resistance {
            base_force = ramp(brake, s.brake_deadzone, s.brake_baseline_force, s.brake_max_force, s.brake_curve, s.brake_wall_engage_at);
        }
        if handbrake {
            base_force = base_force.saturating_add(s.handbrake_bonus);
        }

        self.l2_in_wall = wall_state(brake, self.l2_in_wall, s.brake_wall_engage_at, s.brake_wall_release_at);

        let mut abs_active = false;
        if s.enable_abs && brake >= s.abs_brake_threshold && (t.speed * 3.6) >= s.abs_min_speed_kmh {
            let max_slip = max_driven_wheels(2, t.tire_slip_ratio_fl.abs(), t.tire_slip_ratio_fr.abs(), t.tire_slip_ratio_rl.abs(), t.tire_slip_ratio_rr.abs());
            let max_c_slip = max_driven_wheels(2, t.tire_combined_slip_fl.abs(), t.tire_combined_slip_fr.abs(), t.tire_combined_slip_rl.abs(), t.tire_combined_slip_rr.abs());
            
            if max_slip >= s.abs_slip_ratio_threshold || max_c_slip >= s.abs_combined_slip_threshold {
                self.abs_until = Some(now + std::time::Duration::from_millis(80));
            }
        }

        if let Some(until) = self.abs_until {
            if now < until {
                abs_active = true;
            }
        }

        if abs_active {
            return build_vibrating_resistance(base_force, s.abs_freq, s.abs_amp, self.l2_in_wall, s.wall_zones);
        }

        if self.l2_in_wall {
            return build_wall(s.wall_zones);
        }

        if s.enable_brake_static_wall {
            return build_brake_walls(s.brake_static_wall_at, s.brake_static_wall_force, s.wall_zones);
        }

        if !s.enable_brake_resistance {
            return if handbrake { rigid(s.handbrake_bonus) } else { off() };
        }

        rigid(base_force)
    }

    fn r2(&mut self, t: &ForzaTelemetry, s: &Config, now: Instant) -> TriggerMode {
        let accel = t.accel;

        if s.enable_gear_shift {
            if let Some(burst) = self.shift_burst(s, now, accel, s.throttle_wall_engage_at) {
                return burst;
            }
        }

        let mut base_force = 0;
        if s.enable_throttle_resistance {
            base_force = ramp(accel, s.accel_deadzone, s.throttle_baseline_force, s.throttle_max_force, s.throttle_curve, s.throttle_wall_engage_at);
        }

        self.r2_in_wall = wall_state(accel, self.r2_in_wall, s.throttle_wall_engage_at, s.throttle_wall_release_at);

        if s.enable_rev_limiter && accel >= s.accel_deadzone {
            let max_rpm = if t.max_rpm > 0.0 { t.max_rpm } else { 1.0 };
            let rpm_r = t.rpm / max_rpm;
            if rpm_r > s.rev_limit_ratio {
                self.rev_until = Some(now + std::time::Duration::from_secs_f32(s.rev_limit_hold_ms / 1000.0));
            }
        }

        let mut rev_active = false;
        if let Some(until) = self.rev_until {
            if now < until {
                rev_active = true;
            }
        }

        let mut wheelspin_active = false;
        let mut wheelspin_freq = 0;
        let mut wheelspin_amp = 0;

        if s.enable_wheelspin_buzz && (t.speed * 3.6) >= 10.0 && accel >= s.accel_deadzone {
            let max_slip = max_driven_wheels(t.drive_train, t.tire_slip_ratio_fl, t.tire_slip_ratio_fr, t.tire_slip_ratio_rl, t.tire_slip_ratio_rr);

            if max_slip >= 1.2 {
                self.wheelspin_until = Some(now + std::time::Duration::from_millis(80));
            }
        }

        if let Some(until) = self.wheelspin_until {
            if now < until {
                wheelspin_active = true;

                let max_pud = max_driven_wheels(t.drive_train, t.wheel_in_puddle_depth_fl, t.wheel_in_puddle_depth_fr, t.wheel_in_puddle_depth_rl, t.wheel_in_puddle_depth_rr);
                let max_rum = max_driven_wheels(t.drive_train, t.surface_rumble_fl.abs(), t.surface_rumble_fr.abs(), t.surface_rumble_rl.abs(), t.surface_rumble_rr.abs());

                if max_pud > 0.0 {
                    wheelspin_freq = 100;
                    wheelspin_amp = (s.wheelspin_amp / 2).max(1);
                } else if max_rum > 0.30 { 
                    wheelspin_freq = 20;
                    wheelspin_amp = 15;
                } else if max_rum > 0.10 { 
                    wheelspin_freq = 60;
                    wheelspin_amp = 8;
                } else {
                    wheelspin_freq = 100;
                    wheelspin_amp = s.wheelspin_amp;
                }
            }
        }

        if rev_active {
            return build_vibrating_resistance(base_force, s.rev_limit_freq, s.rev_limit_amp, self.r2_in_wall, s.wall_zones);
        }

        if wheelspin_active {
            return build_vibrating_resistance(base_force, wheelspin_freq, wheelspin_amp, self.r2_in_wall, s.wall_zones);
        }

        if self.r2_in_wall {
            return build_wall(s.wall_zones);
        }

        if !s.enable_throttle_resistance {
            return off();
        }

        rigid(base_force)
    }
}
