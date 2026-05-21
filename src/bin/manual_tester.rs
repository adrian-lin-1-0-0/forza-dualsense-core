use std::io::{self, Write};
use std::net::UdpSocket;
use std::time::Duration;

const TELEMETRY_PORT: u16 = 5300;

fn base_telemetry() -> [u8; 324] {
    let mut telemetry = [0u8; 324];
    telemetry[0..4].copy_from_slice(&1i32.to_le_bytes()); // is_running
    telemetry
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Forza DualSense Manual Tester ---");
    println!("Make sure `forza-dualsense-core` is running in another terminal!");
    println!("Sending telemetry to 127.0.0.1:{}", TELEMETRY_PORT);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let target = format!("127.0.0.1:{}", TELEMETRY_PORT);

    loop {
        println!("\nSelect an effect to test on your physical controller:");
        println!("1. Acceleration (Full Gas, R2 should get firm then hit wall)");
        println!("2. ABS Brake (Full Brake + High Slip, L2 should pulse)");
        println!("3. Engine RPM (High RPM, R2 should buzz)");
        println!("4. Wheelspin (Gravel, R2 should pulse strongly)");
        println!("5. Handbrake (L2 should become extra stiff)");
        println!("6. Gear Shift (Quick burst on L2/R2)");
        println!("7. Clear (Reset all pedals to zero)");
        println!("0. Exit");
        print!("\nEnter choice: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        let mut t = base_telemetry();
        
        match choice {
            "1" => {
                println!(">> Sending Full Acceleration (holding for 3s)...");
                t[315] = 255; // Accel
                for _ in 0..100 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
            }
            "2" => {
                println!(">> Sending ABS Brake (holding for 3s)...");
                t[316] = 255; // Brake
                t[256..260].copy_from_slice(&20.0f32.to_le_bytes()); // speed > 15
                t[84..88].copy_from_slice(&1.5f32.to_le_bytes()); // slip fl
                for _ in 0..100 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
            }
            "3" => {
                println!(">> Sending Engine RPM (holding for 3s)...");
                t[315] = 128; // Mid Accel
                t[8..12].copy_from_slice(&8000.0f32.to_le_bytes()); // max_rpm
                t[16..20].copy_from_slice(&7600.0f32.to_le_bytes()); // current rpm > 93%
                for _ in 0..100 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
            }
            "4" => {
                println!(">> Sending Wheelspin on Gravel (holding for 3s)...");
                t[315] = 255; // Accel
                t[256..260].copy_from_slice(&20.0f32.to_le_bytes()); // speed
                t[92..96].copy_from_slice(&2.0f32.to_le_bytes()); // tire_slip_ratio_rl
                t[156..160].copy_from_slice(&0.5f32.to_le_bytes()); // surface_rumble_rl
                for _ in 0..100 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
            }
            "5" => {
                println!(">> Sending Handbrake (holding for 3s)...");
                t[316] = 100; // Brake mid
                t[318] = 255; // Handbrake
                for _ in 0..100 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
            }
            "6" => {
                println!(">> Sending Gear Shift Burst...");
                // Send idle first
                t[315] = 255;
                t[319] = 1; // Gear 1
                for _ in 0..5 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
                
                // Shift to gear 2
                t[319] = 2; // Gear 2
                for _ in 0..10 {
                    socket.send_to(&t, &target)?;
                    std::thread::sleep(Duration::from_millis(30));
                }
            }
            "7" => {
                println!(">> Clearing pedals...");
                socket.send_to(&t, &target)?;
            }
            "0" => {
                println!("Exiting.");
                break;
            }
            _ => {
                println!("Invalid choice.");
            }
        }
    }

    Ok(())
}
