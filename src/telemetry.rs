use bytemuck::{Pod, Zeroable};
use std::net::UdpSocket;
use std::time::Duration;

pub const PACKET_SIZE: usize = 324;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ForzaTelemetry {
    pub is_running: i32,           // 0
    pub timestamp_ms: u32,         // 4
    pub max_rpm: f32,              // 8
    pub idle_rpm: f32,             // 12
    pub rpm: f32,                  // 16
    pub accel_x: f32,              // 20
    pub accel_y: f32,              // 24
    pub accel_z: f32,              // 28
    pub velocity_x: f32,           // 32
    pub velocity_y: f32,           // 36
    pub velocity_z: f32,           // 40
    pub angular_velocity_x: f32,   // 44
    pub angular_velocity_y: f32,   // 48
    pub angular_velocity_z: f32,   // 52
    pub yaw: f32,                  // 56
    pub pitch: f32,                // 60
    pub roll: f32,                 // 64
    pub norm_suspension_travel_fl: f32, // 68
    pub norm_suspension_travel_fr: f32, // 72
    pub norm_suspension_travel_rl: f32, // 76
    pub norm_suspension_travel_rr: f32, // 80
    pub tire_slip_ratio_fl: f32,   // 84
    pub tire_slip_ratio_fr: f32,   // 88
    pub tire_slip_ratio_rl: f32,   // 92
    pub tire_slip_ratio_rr: f32,   // 96
    pub wheel_rotation_speed_fl: f32, // 100
    pub wheel_rotation_speed_fr: f32, // 104
    pub wheel_rotation_speed_rl: f32, // 108
    pub wheel_rotation_speed_rr: f32, // 112
    pub wheel_on_rumble_strip_fl: i32, // 116
    pub wheel_on_rumble_strip_fr: i32, // 120
    pub wheel_on_rumble_strip_rl: i32, // 124
    pub wheel_on_rumble_strip_rr: i32, // 128
    pub wheel_in_puddle_depth_fl: f32, // 132
    pub wheel_in_puddle_depth_fr: f32, // 136
    pub wheel_in_puddle_depth_rl: f32, // 140
    pub wheel_in_puddle_depth_rr: f32, // 144
    pub surface_rumble_fl: f32,    // 148
    pub surface_rumble_fr: f32,    // 152
    pub surface_rumble_rl: f32,    // 156
    pub surface_rumble_rr: f32,    // 160
    pub tire_slip_angle_fl: f32,   // 164
    pub tire_slip_angle_fr: f32,   // 168
    pub tire_slip_angle_rl: f32,   // 172
    pub tire_slip_angle_rr: f32,   // 176
    pub tire_combined_slip_fl: f32, // 180
    pub tire_combined_slip_fr: f32, // 184
    pub tire_combined_slip_rl: f32, // 188
    pub tire_combined_slip_rr: f32, // 192
    pub suspension_travel_meters_fl: f32, // 196
    pub suspension_travel_meters_fr: f32, // 200
    pub suspension_travel_meters_rl: f32, // 204
    pub suspension_travel_meters_rr: f32, // 208
    pub car_ordinal: i32,          // 212
    pub car_class: i32,            // 216
    pub car_performance_index: i32, // 220
    pub drive_train: i32,          // 224
    pub num_cylinders: i32,        // 228
    pub padding1: [u8; 12],        // 232 to 244
    pub position_x: f32,           // 244
    pub position_y: f32,           // 248
    pub position_z: f32,           // 252
    pub speed: f32,                // 256 (m/s)
    pub power: f32,                // 260
    pub torque: f32,               // 264
    pub tire_temp_fl: f32,         // 268
    pub tire_temp_fr: f32,         // 272
    pub tire_temp_rl: f32,         // 276
    pub tire_temp_rr: f32,         // 280
    pub boost: f32,                // 284
    pub fuel: f32,                 // 288
    pub distance_traveled: f32,    // 292
    pub best_lap_time: f32,        // 296
    pub last_lap_time: f32,        // 300
    pub current_lap_time: f32,     // 304
    pub current_race_time: f32,    // 308
    pub lap_number: u16,           // 312
    pub race_position: u8,         // 314
    pub accel: u8,                 // 315
    pub brake: u8,                 // 316
    pub clutch: u8,                // 317
    pub handbrake: u8,             // 318
    pub gear: u8,                  // 319
    pub steer: i8,                 // 320
    pub normalized_driving_line: i8, // 321
    pub normalized_ai_brake_difference: i8, // 322
    pub padding2: u8,              // 323 (making it 324 bytes)
}

pub struct UdpListener {
    sock: UdpSocket,
    buffer: [u8; 1500],
}

impl UdpListener {
    pub fn new(host: &str, port: u16, timeout_secs: f32) -> std::io::Result<Self> {
        let addr = format!("{}:{}", host, port);
        let sock = UdpSocket::bind(&addr)?;
        
        let timeout = Duration::from_secs_f32(timeout_secs);
        sock.set_read_timeout(Some(timeout))?;
        // Set non-blocking to false initially, we use read_timeout instead.
        sock.set_nonblocking(false)?;

        Ok(Self {
            sock,
            buffer: [0u8; 1500],
        })
    }

    /// Receives the latest packet, draining any queued stale packets.
    pub fn recv_latest(&mut self) -> Option<&ForzaTelemetry> {
        let mut latest_size;
        
        // Block for the first packet or timeout
        match self.sock.recv_from(&mut self.buffer) {
            Ok((size, _)) => {
                latest_size = size;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                return None;
            }
            Err(_) => return None,
        }

        // Drain the socket to get the most recent packet
        self.sock.set_nonblocking(true).unwrap();
        loop {
            match self.sock.recv_from(&mut self.buffer) {
                Ok((size, _)) => latest_size = size,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break, // Other errors, just break and use the last successfully read packet
            }
        }
        self.sock.set_nonblocking(false).unwrap();

        if latest_size == PACKET_SIZE {
            // Zero-copy cast from bytes to ForzaTelemetry struct
            let telemetry: &ForzaTelemetry = bytemuck::from_bytes(&self.buffer[..PACKET_SIZE]);
            Some(telemetry)
        } else {
            None
        }
    }
}
