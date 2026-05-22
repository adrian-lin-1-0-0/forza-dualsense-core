use hidapi::{HidApi, HidDevice};
use std::time::{Instant, Duration};
use std::net::UdpSocket;

const VENDOR_ID: u16 = 0x054C;
const PRODUCT_IDS: [u16; 2] = [0x0CE6, 0x0DF2]; // DualSense, DualSense Edge

#[derive(Debug, PartialEq, Eq)]
pub enum TriggerMode {
    Off,
    Rigid(u8),       // force
    Pulse(u8, u8),   // freq, amp
    Feedback(u16, u32), // active_zones, strength
    PulseAB(u16, u32, u8), // active_zones, strength, freq
}

struct Layout {
    rid: u8,
    flags: usize,
    r: usize,
    l: usize,
    size: usize,
    bt: bool,
}

const USB: Layout = Layout { rid: 0x02, flags: 1, r: 11, l: 22, size: 64, bt: false };
const BT: Layout = Layout { rid: 0x31, flags: 2, r: 12, l: 23, size: 78, bt: true };

pub struct DualSense {
    api: Option<HidApi>,
    device: Option<HidDevice>,
    mock_socket: Option<UdpSocket>,
    mock_port: Option<u16>,
    layout: &'static Layout,
    last_reconnect_attempt: Option<Instant>,
    reconnect_interval: Duration,
}

impl DualSense {
    pub fn new() -> Result<Self, String> {
        let mock_port = std::env::var("MOCK_USB_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok());

        let (api, mock_socket) = if mock_port.is_some() {
            println!("Starting DualSense in MOCK mode.");
            (None, UdpSocket::bind("127.0.0.1:0").ok())
        } else {
            let api = HidApi::new().map_err(|e| format!("Failed to init HidApi: {}", e))?;
            (Some(api), None)
        };

        Ok(Self {
            api,
            device: None,
            mock_socket,
            mock_port,
            layout: &USB,
            last_reconnect_attempt: None,
            reconnect_interval: Duration::from_secs(2),
        })
    }

    pub fn is_connected(&self) -> bool {
        self.device.is_some() || self.mock_socket.is_some()
    }

    pub fn try_connect(&mut self) -> bool {
        if self.is_connected() {
            return true;
        }

        if let Some(last) = self.last_reconnect_attempt {
            if last.elapsed() < self.reconnect_interval {
                return false;
            }
        }
        self.last_reconnect_attempt = Some(Instant::now());

        if self.mock_socket.is_some() {
            return true; // Already "connected" in mock mode
        }

        if let Some(api) = &mut self.api {
            let _ = api.refresh_devices();
            for dev_info in api.device_list() {
                if dev_info.vendor_id() == VENDOR_ID && PRODUCT_IDS.contains(&dev_info.product_id()) {
                    if dev_info.usage_page() == 1 && dev_info.usage() == 5 {
                        if let Ok(dev) = dev_info.open_device(api) {
                            let _ = dev.set_blocking_mode(false);
                            self.device = Some(dev);
                            
                            let path = dev_info.path().to_string_lossy().to_uppercase();
                            if path.contains("BTHENUM") || path.contains("BLUETOOTH") {
                                self.layout = &BT;
                            } else {
                                self.layout = &USB;
                            }
                            
                            println!("DualSense connected ({})", if self.layout.bt { "BT" } else { "USB" });
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn update(&mut self, left: &TriggerMode, right: &TriggerMode) -> Result<(), String> {
        if !self.is_connected() {
            return Ok(());
        }

        let mut buf = vec![0u8; self.layout.size];
        buf[0] = self.layout.rid;
        if self.layout.bt {
            buf[1] = 0x02;
        }

        // TRIG_FLAGS = 0x04 | 0x08
        buf[self.layout.flags] = 0x0C;

        self.apply_mode(self.layout.r, right, &mut buf);
        self.apply_mode(self.layout.l, left, &mut buf);

        if self.layout.bt {
            let crc = crc32fast::hash(&buf[..74]);
            buf[74..78].copy_from_slice(&crc.to_le_bytes());
        }

        if let Some(sock) = &self.mock_socket {
            if let Some(port) = self.mock_port {
                let _ = sock.send_to(&buf, format!("127.0.0.1:{}", port));
            }
            return Ok(());
        }

        if let Some(dev) = &self.device {
            match dev.write(&buf) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("DualSense disconnected: {}", e);
                    self.device = None;
                    Err(e.to_string())
                }
            }
        } else {
            Ok(())
        }
    }

    fn apply_mode(&self, pos: usize, mode: &TriggerMode, buf: &mut [u8]) {
        match mode {
            TriggerMode::Off => {
                buf[pos] = 0x05;
                buf[pos + 1..pos + 11].fill(0);
            }
            TriggerMode::Rigid(force) => {
                buf[pos] = 0x01;
                buf[pos + 1] = 0;
                buf[pos + 2] = *force;
                buf[pos + 3..pos + 11].fill(0);
            }
            TriggerMode::Pulse(freq, amp) => {
                buf[pos] = 0x06;
                buf[pos + 1] = *freq;
                buf[pos + 2] = *amp;
                buf[pos + 3..pos + 11].fill(0);
            }
            TriggerMode::Feedback(active, force) => {
                buf[pos] = 0x21;
                buf[pos + 1..pos + 3].copy_from_slice(&active.to_le_bytes());
                buf[pos + 3..pos + 7].copy_from_slice(&force.to_le_bytes());
                buf[pos + 7..pos + 11].fill(0);
            }
            TriggerMode::PulseAB(active, force, freq) => {
                buf[pos] = 0x26;
                buf[pos + 1..pos + 3].copy_from_slice(&active.to_le_bytes());
                buf[pos + 3..pos + 7].copy_from_slice(&force.to_le_bytes());
                buf[pos + 7] = *freq;
                buf[pos + 8..pos + 11].fill(0);
            }
        }
    }
}
