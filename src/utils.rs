pub fn amp_to_strength(amp_byte: u8) -> u8 {
    (amp_byte / 32 + 1).clamp(1, 8)
}

pub fn ramp(value: u8, deadzone: u8, baseline: u8, max_force: u8, curve: f32, ceiling: u8) -> u8 {
    if value < deadzone {
        return baseline;
    }
    let r = (value - deadzone) as f32 / ((ceiling.saturating_sub(deadzone)).max(1)) as f32;
    let r = r.min(1.0);
    let res = baseline as f32 + (max_force as f32 - baseline as f32) * r.powf(curve);
    res as u8
}

pub fn wall_state(value: u8, engaged: bool, engage_at: u8, release_at: u8) -> bool {
    if engaged {
        value >= release_at
    } else {
        value >= engage_at
    }
}

pub fn max_driven_wheels(drive_train: i32, fl: f32, fr: f32, rl: f32, rr: f32) -> f32 {
    match drive_train {
        0 => fl.max(fr),
        1 => rl.max(rr),
        _ => fl.max(fr).max(rl).max(rr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amp_to_strength() {
        assert_eq!(amp_to_strength(0), 1);
        assert_eq!(amp_to_strength(255), 8);
        assert_eq!(amp_to_strength(32), 2);
    }

    #[test]
    fn test_ramp() {
        assert_eq!(ramp(10, 20, 50, 100, 1.0, 200), 50); // Below deadzone
        assert_eq!(ramp(200, 0, 0, 100, 1.0, 200), 100); // Max force
    }

    #[test]
    fn test_wall_state() {
        assert_eq!(wall_state(100, false, 200, 100), false);
        assert_eq!(wall_state(250, false, 200, 100), true);
        assert_eq!(wall_state(150, true, 200, 100), true);
        assert_eq!(wall_state(50, true, 200, 100), false);
    }

    #[test]
    fn test_max_driven_wheels() {
        assert_eq!(max_driven_wheels(0, 1.0, 2.0, 3.0, 4.0), 2.0); // FWD
        assert_eq!(max_driven_wheels(1, 1.0, 2.0, 3.0, 4.0), 4.0); // RWD
        assert_eq!(max_driven_wheels(2, 1.0, 2.0, 3.0, 4.0), 4.0); // AWD
    }
}
