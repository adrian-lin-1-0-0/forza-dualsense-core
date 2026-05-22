use std::process::Command;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::sleep;

fn find_free_udp_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .expect("Failed to bind an ephemeral UDP socket")
        .local_addr()
        .expect("Failed to read local UDP socket address")
        .port()
}

async fn run_test_case(
    telemetry_sender: &UdpSocket,
    usb_receiver: &UdpSocket,
    telemetry_port: u16,
    name: &str,
    telemetry: &[u8; 324],
    expected_l: Option<u8>,
    expected_r: Option<u8>,
) {
    println!("--- Running Test: {} ---", name);
    let mut buf = vec![0u8; 1024];

    // Drain pending packets
    while let Ok(_) = usb_receiver.try_recv_from(&mut buf) {}

    // Send telemetry
    telemetry_sender
        .send_to(telemetry, format!("127.0.0.1:{}", telemetry_port))
        .await
        .unwrap();

    let timeout_res =
        tokio::time::timeout(Duration::from_secs(1), usb_receiver.recv_from(&mut buf)).await;
    match timeout_res {
        Ok(Ok((_len, _addr))) => {
            let l_trigger_mode = buf[22];
            let r_trigger_mode = buf[11];

            if let Some(l) = expected_l {
                assert_eq!(
                    l_trigger_mode, l,
                    "Test {}: Expected L mode {:#04x}, got {:#04x}",
                    name, l, l_trigger_mode
                );
            }
            if let Some(r) = expected_r {
                assert_eq!(
                    r_trigger_mode, r,
                    "Test {}: Expected R mode {:#04x}, got {:#04x}",
                    name, r, r_trigger_mode
                );
            }
        }
        Ok(Err(e)) => panic!("Error receiving USB data: {}", e),
        Err(_) => panic!("Timeout waiting for USB data in test: {}", name),
    }

    sleep(Duration::from_millis(200)).await;
}

#[tokio::test]
async fn test_e2e_scenarios() {
    let telemetry_port = find_free_udp_port();
    let usb_receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mock_usb_port = usb_receiver.local_addr().unwrap().port();

    let mut child = Command::new(env!("CARGO_BIN_EXE_forza-dualsense-core"))
        .env("MOCK_TELEMETRY_PORT", telemetry_port.to_string())
        .env("MOCK_USB_PORT", mock_usb_port.to_string())
        .spawn()
        .expect("Failed to start Rust core engine.");

    let telemetry_sender = UdpSocket::bind("0.0.0.0:0").await.unwrap();

    sleep(Duration::from_millis(1000)).await;

    let mut buf = vec![0u8; 1024];
    for _ in 1..=2 {
        tokio::time::timeout(Duration::from_secs(2), usb_receiver.recv_from(&mut buf))
            .await
            .expect("Timeout waiting for startup pulse")
            .unwrap();
    }

    let base_telemetry = || -> [u8; 324] {
        let mut telemetry = [0u8; 324];
        telemetry[0..4].copy_from_slice(&1i32.to_le_bytes()); // is_running = true
        telemetry
    };

    let mut telemetry1 = base_telemetry();
    telemetry1[315] = 255; // Accel
    telemetry1[316] = 0; // Brake
    // 0x01 (Rigid) for L, 0x21 (Feedback/Wall) for R
    run_test_case(
        &telemetry_sender,
        &usb_receiver,
        telemetry_port,
        "1. Acceleration",
        &telemetry1,
        Some(0x01),
        Some(0x21),
    )
    .await;

    let mut telemetry2 = base_telemetry();
    telemetry2[315] = 0; // Accel
    telemetry2[316] = 255; // Brake
    telemetry2[256..260].copy_from_slice(&10.0f32.to_le_bytes()); // speed (10 m/s = 36 km/h) - Need speed for ABS!
    telemetry2[84..88].copy_from_slice(&1.5f32.to_le_bytes()); // tire_slip_ratio_fl
    telemetry2[88..92].copy_from_slice(&1.5f32.to_le_bytes()); // tire_slip_ratio_fr
    // 0x26 (PulseAB) for ABS on L, 0x01 (Rigid) on R
    run_test_case(
        &telemetry_sender,
        &usb_receiver,
        telemetry_port,
        "2. ABS Brake",
        &telemetry2,
        Some(0x26),
        Some(0x01),
    )
    .await;

    let mut telemetry3 = base_telemetry();
    telemetry3[315] = 255; // Accel
    telemetry3[8..12].copy_from_slice(&8000.0f32.to_le_bytes()); // max_rpm
    telemetry3[16..20].copy_from_slice(&7500.0f32.to_le_bytes()); // current rpm
    // 0x01 (Rigid) for L, 0x26 (PulseAB) for R Rev Limiter
    run_test_case(
        &telemetry_sender,
        &usb_receiver,
        telemetry_port,
        "3. Engine RPM",
        &telemetry3,
        Some(0x01),
        Some(0x26),
    )
    .await;

    let mut telemetry6 = base_telemetry();
    telemetry6[315] = 255; // Accel
    telemetry6[224..228].copy_from_slice(&2i32.to_le_bytes()); // drive_train = 2 (AWD)
    telemetry6[256..260].copy_from_slice(&10.0f32.to_le_bytes()); // speed
    telemetry6[92..96].copy_from_slice(&2.0f32.to_le_bytes()); // tire_slip_ratio_rl
    telemetry6[140..144].copy_from_slice(&1.0f32.to_le_bytes()); // wheel_in_puddle_depth_rl
    // 0x01 (Rigid) for L, 0x26 (PulseAB) for R Wheelspin
    run_test_case(
        &telemetry_sender,
        &usb_receiver,
        telemetry_port,
        "6. Wheelspin",
        &telemetry6,
        Some(0x01),
        Some(0x26),
    )
    .await;

    // Clean up
    child.kill().unwrap();
    child.wait().unwrap();
}
