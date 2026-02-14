# Gjallarhorn

[![Crates.io](https://img.shields.io/crates/v/gjallarhorn?style=flat-square)](https://crates.io/crates/gjallarhorn)
[![Documentation](https://img.shields.io/docsrs/gjallarhorn?style=flat-square)](https://docs.rs/gjallarhorn)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/gjallarhorn?style=flat-square)](https://crates.io/crates/gjallarhorn)

**Gjallarhorn** is a modern, lightweight, and cross-platform system resource monitor written in **Rust** using the **Slint** UI toolkit. It provides real-time visualization of your system's performance metrics with a sleek and customizable interface.

## Architecture: Privilege Separation

Gjallarhorn uses a **Client-Worker** architecture to securely gather privileged hardware information (like SMART disk health, serial numbers, and DMI data) without running the entire GUI as root.

- **GUI (Client)**: Runs as your standard user, ensuring full compatibility with Wayland and X11 environments.
- **Worker (Privileged)**: A background process spawned via `pkexec` when the application starts. It gathers sensitive data and streams it to the GUI.
  - *Note: You will be prompted for your password once upon launch to authorize this worker.*

## Features

- **Real-Time Monitoring**:
  - **CPU**: Per-core usage history, model name, architecture, and frequency.
  - **Memory (RAM)**: Total/Used capacity, and detailed specs (Type, Speed, Module Count, Form Factor via `dmidecode`).
  - **GPU**: NVIDIA GPU stats (Utilization, Memory, Power Draw, Temp, Fan Speed) via `nvml-wrapper`.
  - **Storage**: Disk usage, plus detailed health info (SMART status, Model, Serial, Firmware, Interface Type) via `smartctl`.
  - **Network**: Real-time traffic (Upload/Download) and interface details (IPs, MAC, Link Speed).

- **Customizable UI**:
  - **Dark/Light Mode**: Toggle themes instantly.
  - **Refresh Rate**: Adjust from **100ms** to **2000ms**.
  - **Color Themes**: Customize chart colors for CPU, RAM, GPU, and Network.
  - **Persistent Settings**: Preferences are saved automatically.

- **Modern Design**:
  - Responsive Slint-based UI with smooth animations and rounded corners.
  - Tabbed interface for organized data viewing.

## Installation

### Prerequisites

- **Rust Toolchain**: [Install Rust](https://www.rust-lang.org/tools/install)
- **System Dependencies**:
  - **Slint Dependencies**: `libfontconfig-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev` (Linux).
  - **run-time**: `pkexec` (usually installed by default on desktop Linux).
  - **smartmontools**: For disk health stats (`sudo apt install smartmontools`).
  - **dmidecode**: For memory specs (`sudo apt install dmidecode`).

### Production Install (Recommended)

Gjallarhorn includes a `Makefile` for easy system-wide installation, which properly sets up the desktop entry.

```bash
git clone https://github.com/arunkumar-mourougappane/gjallarhorn-rs.git
cd gjallarhorn-rs
make install
```

This will:

1. Compile the release binary.
2. Install `gjallarhorn` to `/usr/local/bin`.
3. Install the `.desktop` file to `/usr/local/share/applications` (making it visible in your app menu).

To uninstall:

```bash
make uninstall
```

### Development Run

```bash
cargo run --release
```

*Note: When running via cargo, the worker spawning might fail if the binary path isn't standard, or `pkexec` might behave differently. The `make install` method is preferred for full feature verification.*

## Usage

1. **Launch**: Open **Gjallarhorn** from your application launcher.
2. **Authorize**: Enter your password when prompted to allow the helper process to check hardware health.
   - *If you cancel/deny, the app will still run, but some detailed stats (SMART, Serial Nums) will be unavailable.*
3. **Monitor**:
   - **Overview**: CPU/RAM/GPU/Net summary graphs.
   - **Hardware Tabs**: Click the tabs at the top (CPU, Memory, Storage, GPU, Network) for detailed tables and specs.
4. **Preferences**: File > Preferences to tweak colors and refresh rates.

## Configuration

Settings are stored in: `~/.config/gjallarhorn/config.json`.

## Tech Stack

- **Language**: Rust
- **UI Framework**: [Slint](https://slint.dev/)
- **System Info**: `sysinfo`
- **GPU Info**: `nvml-wrapper`
- **Privilege Mgmt**: `pkexec` (PolicyKit)
- **Serialization**: `serde` & `serde_json`
