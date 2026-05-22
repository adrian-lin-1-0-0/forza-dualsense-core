use std::process::{Child, Command};

pub struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    pub fn spawn(mut command: Command) -> Self {
        let child = command.spawn().expect("Failed to start Rust core engine.");
        Self { child }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}

pub fn spawn_core_engine(telemetry_port: u16, mock_usb_port: u16) -> ChildGuard {
    ChildGuard::spawn({
        let mut command = Command::new(env!("CARGO_BIN_EXE_forza-dualsense-core"));
        command
            .env("MOCK_TELEMETRY_PORT", telemetry_port.to_string())
            .env("MOCK_USB_PORT", mock_usb_port.to_string());
        command
    })
}
