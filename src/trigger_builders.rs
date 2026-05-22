use crate::controller::TriggerMode;
use crate::utils::amp_to_strength;

pub fn off() -> TriggerMode {
    TriggerMode::Off
}

pub fn rigid(force: u8) -> TriggerMode {
    TriggerMode::Rigid(force)
}

pub fn vibration(freq: u8, amp: u8) -> TriggerMode {
    TriggerMode::Pulse(freq, amp)
}

fn force_to_feedback_strength(force: u8) -> u8 {
    if force == 0 {
        0
    } else {
        amp_to_strength(force)
    }
}

fn vibration_to_feedback_strength(amp: u8) -> u8 {
    if amp == 0 {
        0
    } else {
        amp.saturating_add(3).saturating_div(4).clamp(1, 8)
    }
}

fn encode_zones(zones: &[u8; 10]) -> (u16, u32) {
    let mut active = 0u16;
    let mut strength = 0u32;

    for (i, &zone_strength) in zones.iter().enumerate() {
        let zone_strength = zone_strength.clamp(0, 8);
        if zone_strength > 0 {
            active |= 1 << i;
            strength |= ((zone_strength - 1) as u32) << (3 * i);
        }
    }

    (active, strength)
}

fn pulse_ab_from_zones(zones: &[u8; 10], freq: u8) -> TriggerMode {
    let (active, strength) = encode_zones(zones);
    TriggerMode::PulseAB(active, strength, freq)
}

pub fn vibration_wall(amp: u8, freq: u8, wall_zones: u8) -> TriggerMode {
    let a = amp.clamp(1, 8);
    let w = wall_zones.clamp(1, 9) as usize;
    let mut zones = [a; 10];
    for i in (10 - w)..10 {
        zones[i] = 8;
    }

    pulse_ab_from_zones(&zones, freq)
}

pub fn build_vibrating_resistance(
    base_force: u8,
    freq: u8,
    amp: u8,
    in_wall: bool,
    wall_zones: u8,
) -> TriggerMode {
    let base_strength = force_to_feedback_strength(base_force);
    let buzz_strength = vibration_to_feedback_strength(amp);
    let combined_strength = base_strength.max(buzz_strength);

    let mut zones = [combined_strength; 10];
    if in_wall {
        let w = wall_zones.clamp(1, 9) as usize;
        for i in (10 - w)..10 {
            zones[i] = 8;
        }
    }

    pulse_ab_from_zones(&zones, freq)
}

fn feedback(zones: &[u8; 10]) -> TriggerMode {
    let (active, force) = encode_zones(zones);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pulse_ab_zone_strength(mode: TriggerMode) -> u8 {
        match mode {
            TriggerMode::PulseAB(_, force, _) => ((force & 0b111) as u8) + 1,
            other => panic!("expected PulseAB, got {:?}", other),
        }
    }

    #[test]
    fn vibrating_resistance_preserves_wheelspin_tiers() {
        let medium = pulse_ab_zone_strength(build_vibrating_resistance(0, 60, 8, false, 2));
        let high = pulse_ab_zone_strength(build_vibrating_resistance(0, 20, 15, false, 2));

        assert!(
            high > medium,
            "expected high rumble tier to produce stronger feedback"
        );
    }
}
