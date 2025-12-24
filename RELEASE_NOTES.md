# Gjallarhorn v0.1.0 - Initial Release 🎉

We're excited to announce the first release of **Gjallarhorn**, a modern, cross-platform system resource monitor built with Rust and Slint!

## 🚀 What is Gjallarhorn?

Gjallarhorn is a lightweight, real-time system resource monitor that provides beautiful visualizations of your computer's performance metrics. With support for CPU, Memory, GPU, and Network monitoring, Gjallarhorn gives you complete visibility into your system's health.

## ✨ Key Features

### 📊 Comprehensive Hardware Monitoring

- **CPU**: Per-core usage tracking with 60-second historical graphs
- **Memory**: Real-time RAM usage visualization
- **GPU**: NVIDIA GPU compute load and VRAM monitoring (via NVML)
- **Network**: All network interfaces with upload/download rates and totals

### 🎨 Beautiful & Customizable UI

- Modern, responsive interface built with Slint
- **Dark Mode**: Seamless theme switching
- **Color Customization**: Personalize chart colors for CPU, RAM, GPU, and Network
- **CPU Color Modes**: Choose between uniform color or distinct per-core colors
- **Persistent Settings**: All preferences save automatically

### ⚡ Performance & Quality

- Optimized for smooth 60 FPS rendering
- Minimal resource footprint
- Release builds recommended for best performance
- All code passes clippy strict linting
- Comprehensive documentation

## 📦 Installation

### From crates.io (Recommended)

```bash
cargo install gjallarhorn
```

### From Source

```bash
git clone https://github.com/arunkumar-mourougappane/gjallarhorn-rs.git
cd gjallarhorn-rs
cargo install --path .
```

## 🎯 Usage

Simply run:

```bash
gjallarhorn
```

Then navigate using:

- **Tabs**: Switch between CPU, RAM, GPU, and Network views
- **File Menu**: Access Preferences and Quit
- **Help Menu**: View About information

## 🛠️ Tech Stack

- **Language**: Rust (Edition 2021)
- **UI Framework**: [Slint](https://slint.dev/) 1.8.0
- **System Info**: sysinfo, nvml-wrapper, default-net
- **Settings**: serde, directories
- **Logging**: log, env_logger

## 📝 Documentation

- [README](https://github.com/arunkumar-mourougappane/gjallarhorn-rs/blob/main/README.md)
- [API Documentation](https://docs.rs/gjallarhorn)
- [Contributing Guide](https://github.com/arunkumar-mourougappane/gjallarhorn-rs/blob/main/CONTRIBUTING.md)

## 🙏 Acknowledgments

Built with ❤️ using:

- [Slint](https://slint.dev/) - Modern UI toolkit for Rust
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) - System information library
- [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper) - NVIDIA GPU monitoring

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Homepage**: <https://github.com/arunkumar-mourougappane/gjallarhorn-rs>
- **crates.io**: <https://crates.io/crates/gjallarhorn>
- **Documentation**: <https://docs.rs/gjallarhorn>
- **Issues**: <https://github.com/arunkumar-mourougappane/gjallarhorn-rs/issues>

---

**Full Changelog**: <https://github.com/arunkumar-mourougappane/gjallarhorn-rs/blob/main/CHANGELOG.md>
