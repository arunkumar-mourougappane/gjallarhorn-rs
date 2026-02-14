# Gjallarhorn v0.2.0 - Production Ready & Privilege Separation 🛡️

We are proud to announce **Gjallarhorn v0.2.0**, a major update focusing on security, architectural improvements, and deeper hardware visibility.

## 🚀 Highlights

### 🔒 Privilege Separation Architecture

Gjallarhorn now employs a robust **Client-Worker** model. The GUI runs as a standard user for maximum compatibility with X11 and Wayland (fixing "cannot open display" errors), while a separate, ephemeral worker process handles privileged data gathering (like SMART health and DMI info) via `pkexec`.

### 📊 Detailed Hardware Information

Five new sub-tabs provide in-depth specifications:

- **CPU**: Cache hierarchy (L1/L2/L3), instruction flags (AVX2, SSE4.2), and virtualization status.
- **Memory**: RAM type (DDR4/5), speed, module count, and channel configuration.
- **Storage**: Drive health (SMART pass/fail), serial numbers, firmware versions, and interface types (NVMe/SATA).
- **GPU**: NVIDIA driver versions, VRAM usage/totals, power draw, temperature, and fan speeds.
- **Network**: MAC addresses, IPv4/IPv6, and link speeds.

### 🛠️ Production Deployment

- **Makefile**: Standardized `make install` and `make uninstall` targets.
- **Desktop Integration**: Includes a `.desktop` file for system menu integration.

---

## 📋 Changelog

### Added

- **Privilege Separation**: Split application into unprivileged GUI and privileged Worker process.
- **Worker Module**: `worker.rs` for headless data gathering.
- **Detailed Info Tabs**: UI expansion for granular hardware data.
- **Network Monitoring**: Per-interface bandwidth tracking.
- **Deployment**: `Makefile` and `gjallarhorn.desktop`.

### Changed

- **Architecture**: `main.rs` now handles `--privileged-worker` flag.
- **Refactoring**: Extensive code cleanup in `monitor.rs` for better modularity and performance.
- **Dependencies**: Updated `sysinfo` usage for better stability.

### Fixed

- **Wayland/X11**: Eliminated root requirement for GUI, ensuring display server compatibility.
- **Code Quality**: Resolved all clippy lints and optimized iterator usage.

---

## 📦 Installation

```bash
# Clone and Install
git clone https://github.com/arunkumar-mourougappane/gjallarhorn-rs.git
cd gjallarhorn-rs
make install
```

## 🤝 Contributors

- Arun Kumar Mourougappane
