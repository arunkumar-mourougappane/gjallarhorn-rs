# Changelog

All notable changes to Gjallarhorn will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-14

### Added

- **Privilege Separation Architecture**: Implemented Client-Worker model to separate GUI (unprivileged) from hardware monitoring (privileged).
- **Detailed Hardware Info Tabs**: Added new sub-tabs for CPU, Memory, Storage, GPU, and Network detailed specifications.
- **Worker Process**: Introduced `worker.rs` handle privileged tasks via `pkexec`.
- **Deployment**: Added `Makefile` for installation and `gjallarhorn.desktop` file.
- **Network Monitoring**: Added detailed per-interface bandwidth tracking.

### Changed

- **Refactoring**: Massive cleanup of `monitor.rs` for modularity and performance.
- **Dependencies**: Updated `sysinfo` integration.
- **Documentation**: Comprehensive updates to `README.md` and inline docs.

### Fixed

- **X11/Wayland Issues**: GUI no longer requires root, fixing display server compatibility.
- **Code Quality**: Resolved all clippy warnings and optimized code paths.

## [0.1.0] - 2025-12-24

### Package Information

- **Name**: Gjallarhorn (rebranded from Heimdall)
- **Naming**: Named after Heimdall's horn in Norse mythology, representing an alert/monitoring system
- **Repository**: <https://github.com/arunkumar-mourougappane/gjallarhorn-rs>

### Added

- **Real-time System Monitoring**
  - CPU monitoring with per-core usage tracking and 60-second history graphs
  - Memory (RAM) monitoring with usage visualization
  - GPU monitoring for NVIDIA GPUs (compute load and VRAM usage)
  - Network monitoring for all active interfaces with real-time upload/download stats

- **Modern User Interface**
  - Clean, responsive Slint-based GUI
  - Tabbed interface for organizing different metrics (CPU, RAM, GPU, Network)
  - Dark Mode support with seamless theme switching
  - Smooth animations and transitions
  - Drop shadows and modern design elements

- **Customization Features**
  - Color customization for all chart types (CPU, RAM, GPU, Network)
  - CPU color modes: Uniform (single color) or Per-Core (hue-based random colors)
  - Persistent settings that save automatically and restore on startup
  - Configuration stored in standard system directories (`~/.config/gjallarhorn/config.json`)

- **Developer Features**
  - Comprehensive inline documentation (rustdoc comments)
  - Modular code structure (settings, utils modules)
  - Structured logging system (info level for debug, error level for release)
  - All clippy warnings resolved
  - Code formatted with rustfmt

- **CI/CD**
  - GitHub Actions workflow for automated builds and testing
  - Runs on ubuntu-latest with all required system dependencies
  - Automated quality checks: cargo fmt, clippy, build (debug + release), and tests
  - Caching for faster CI builds

- **Distribution**
  - Installable as a binary via `cargo install gjallarhorn`
  - Complete crates.io metadata (homepage, documentation, keywords, categories)
  - Professional README with badges and installation instructions
  - Contributing guidelines (CONTRIBUTING.md)
  - MIT License

- **Documentation**
  - Detailed README with features, installation, and usage instructions
  - Module-level documentation for all source files
  - Function-level documentation for public APIs
  - Performance recommendations
  - Contributing guidelines

### Technical Details

- **Language**: Rust (Edition 2021)
- **UI Framework**: Slint 1.8.0
- **Dependencies**:
  - `sysinfo` 0.33.0 for CPU/RAM monitoring
  - `nvml-wrapper` 0.9 for NVIDIA GPU monitoring
  - `default-net` 0.22.0 for network interface detection
  - `serde` & `serde_json` for settings persistence
  - `directories` 6.0.0 for cross-platform config paths
  - `log` & `env_logger` for structured logging

[0.1.0]: https://github.com/arunkumar-mourougappane/gjallarhorn-rs/releases/tag/v0.1.0
