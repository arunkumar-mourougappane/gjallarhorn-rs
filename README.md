# Heimdall-rs

**Heimdall-rs** is a modern, lightweight, and cross-platform system resource monitor written in **Rust** using the **Slint** UI toolkit. It provides real-time visualization of your system's performance metrics with a sleek and customizable interface.

## Features

- **Real-Time Monitoring**:
  - **CPU**: Visualizes per-core usage with support for 60-second history.
  - **Memory (RAM)**: Tracks total system memory usage.
  - **GPU**: Monitors NVIDIA GPU Compute and Memory usage (via `nvml-wrapper`).
  - **Network**: Displays real-time upload/download traffic for all active network interfaces.

- **Customizable UI**:
  - **Dark Mode**: Toggle between Light and Dark themes.
  - **Color Customization**: Fully distinct colors for CPU, RAM, GPU, and Network charts.
  - **CPU Color Modes**: Choose between a "Uniform" single color for all cores or distinct "Random/Hue-based" per-core colors.
  - **Persistent Settings**: Your preferences (colors, theme, mode) are saved automatically and restored on startup.

- **Modern Design**:
  - Responsive layout with Drop Shadows and rounded corners.
  - Smooth animations for buttons and menu transitions.
  - Tabbed interface for organized data viewing.

## Installation

### Prerequisites

- **Rust Toolchain**: Ensure you have Rust installed (`cargo`). [Install Rust](https://www.rust-lang.org/tools/install)
- **System Dependencies**:
  - **Fontconfig** (for Linux font handling)
  - **NVIDIA Drivers** (optional, for GPU stats)

### Building from Source

```bash
git clone https://github.com/arunkumar-mourougappane/heimdall-rs.git
cd heimdall-rs
cargo run --release
```

## Usage

1. **Launch**: Run the application using `cargo run`.
2. **Navigation**: Use the Sidebar to select "Usage" (Monitoring View). The top tabs allow switching between CPU, RAM, GPU, and Network details.
3. **Preferences**:
    - Click **File > Preferences** to open the settings dialog.
    - **Dark Mode**: Switch themes.
    - **CPU Colors**: Toggle "Uniform Color" to use a single color for all cores, or disable it to use persistent random colors for each core.
    - **Other Colors**: Pick custom colors for RAM, GPU, and Network charts.
4. **Quit**: Select **File > Quit** to exit.

## Configuration

Settings are stored in your system's standard configuration directory (e.g., `~/.config/heimdall/config.json` on Linux) and persist across sessions.

## Performance

For the smoothest experience, always compile and run **Heimdall-rs** in **Release** mode.
Debug builds include extensive runtime checks that can significantly slow down the SVG chart generation (parsing ~1000 data points per second).

```bash
cargo run --release
```

or build an optimized binary:

```bash
cargo build --release
./target/release/heimdall-rs
```

## Tech Stack

- **Language**: Rust
- **UI Framework**: [Slint](https://slint.dev/)
- **System Info**: `sysinfo`
- **GPU Info**: `nvml-wrapper`
- **Network Info**: `default-net`
- **Serialization**: `serde` & `serde_json`
