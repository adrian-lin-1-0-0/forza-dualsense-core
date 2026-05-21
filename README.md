# Forza Horizon DualSense (Rust Core Engine)

This project is a high-performance, low-latency Rust rewrite of the core logic for the [Forza Horizon DualSense](https://github.com/HamzaYslmn/Forza-Horizon-DualSense-Python) integration tool. It listens to Forza Horizon's UDP data stream and translates vehicle physics (RPM, brake pressure, tire slip, gears) into dynamic Adaptive Trigger feedback on a PlayStation DualSense controller.

## Features

- **Zero-Allocation Hot Loop**: UDP packet parsing and HID report generation are completely allocation-free.
- **Microsecond Latency**: Uses `bytemuck` to map the raw UDP byte buffer directly into a packed C-struct, bypassing deserialization overhead.
- **Advanced Hardware Control**: Interfaces directly with `hidapi` to manage DualSense states, supporting both USB and Bluetooth (via CRC32 hash signing).
- **Advanced Feedback Effects**:
  - **L2 Brake**: Rigid resistance curve with optional static walls and high-frequency ABS pulse when tire slip is detected.
  - **R2 Throttle**: Feather-light curve switching to a firmware wall, with RPM rev-limiter buzzing and surface-aware longitudinal wheelspin thump (e.g., deeper vibrations on gravel, lighter on tarmac).
  - **Gear Shift Kickback**: Brief, intense burst upon gear changes.

## Prerequisites

- [Rust Toolchain](https://rustup.rs/) (1.75+)
- A PlayStation DualSense controller connected via USB or Bluetooth.
- Forza Horizon 4/5 configured to send Data Out (UDP Telemetry).

## Getting Started

### Using Pre-built Binaries (Linux)

You can download the pre-built Linux binary from the GitHub Releases page:
1. Go to the **[Releases](https://github.com/adrian-lin-1-0-0/forza-dualsense-core/releases)** page on GitHub.
2. Download the `forza-dualsense-core-linux-x86_64.zip` file from the latest release.
3. Extract the zip file, make the binary executable, and run it:
   ```bash
   unzip forza-dualsense-core-linux-x86_64.zip
   chmod +x forza-dualsense-core
   ./forza-dualsense-core
   ```

### Building from Source

1. Clone the repository and navigate into it:
   ```bash
   cd forza-dualsense-core
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run the engine:
   ```bash
   cargo run
   ```

On its first run, the app will auto-generate a `config.toml` file in the same directory. The engine will automatically listen for UDP packets on `127.0.0.1:5300`. 

> **Note:** Make sure Forza Horizon's Data Out is set to target `127.0.0.1` and Port `5300` (or whatever you configure in `config.toml`).

## Configuration (`config.toml`)

The engine watches `config.toml` for changes (currently reloaded upon restart, hot-reload is wired via channels for future GUI integration).

Key tunables include:
- `udp_port`: The port to listen on.
- `brake_max_force` / `throttle_max_force`: The peak resistance force at the end of the pedal travel.
- `enable_abs` / `enable_wheelspin_buzz`: Toggles for physical telemetry effects.

## Testing

### Manual Interactive Tester
To test physical hardware effects without having the game running, you can use the interactive CLI tester. Make sure the core engine is running in one terminal, then open another and run:

```bash
cargo run --bin manual_tester
```
Follow the on-screen prompts to simulate Full Throttle, ABS Braking, Wheelspin, and Gear Shifts directly on your controller.