//! # Main Application Entry Point
//!
//! This file contains the `main` function which orchestrates the Heimdall-rs application.
//! It is responsible for:
//! - Initializing the Slint User Interface.
//! - Loading and applying persistent application settings (`settings` module).
//! - Setting up system hardware monitors (`sysinfo` for CPU/RAM, `nvml-wrapper` for NVIDIA GPUs, `default-net` for Network).
//! - Configuring the main event timer (1s interval) to fetch real-time data.
//! - Updating the UI models (`VecModel`) within the event loop to drive the charts.
//!
//! The application uses a modular structure, delegating configuration to `settings.rs` and utilities to `utils.rs`.

use default_net;
use log::{error, info};
use nvml_wrapper::Nvml;
use slint::{Model, Timer, TimerMode};
use std::rc::Rc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

mod settings;
mod utils;

use settings::AppSettings;
use utils::{brush_to_hex, generate_path, hex_to_color};

include!(env!("SLINT_INCLUDE_GENERATED"));

/// Main entry point of the Heimdall application.
///
/// 1. Initializes the Slint UI.
/// 2. Loads persistent settings.
/// 3. Detects system hardware (CPU, Memory, GPU, Network).
/// 4. Sets up a 1-second timer to refresh system stats.
/// 5. Updates the UI models with real-time data.
fn main() -> Result<(), slint::PlatformError> {
    // Initialize logger with different levels based on build type
    #[cfg(debug_assertions)]
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Error)
        .init();

    let ui = AppWindow::new()?;
    // let ui_handle = ui.as_weak(); // Removed

    // Load Settings
    let mut settings = AppSettings::load();
    let mut settings_changed = false;

    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    // Initial refresh to get CPU count
    system.refresh_cpu_all();
    let cpu_count = system.cpus().len();
    info!("Heimdall detected {} CPU cores (logical)", cpu_count);

    // History for each CPU
    let mut cpus_history: Vec<Vec<f32>> = vec![vec![0.0; 60]; cpu_count];
    let mut memory_history = vec![0.0; 60];

    // Initialize Slint model for CPUs
    let cpu_model = std::rc::Rc::new(slint::VecModel::default());

    for i in 0..cpu_count {
        cpus_history.push(vec![0.0; 60]); // Ensure history exists (though reserved above)

        let color_hex = if i < settings.cpu_core_colors.len() {
            settings.cpu_core_colors[i].clone()
        } else {
            // Generate new random/hue-based color
            let hue = (i as f32 * 360.0 / cpu_count as f32) % 360.0;
            let r = (127.0 + 127.0 * (hue * 0.0174).sin()) as u8;
            let g = (127.0 + 127.0 * ((hue + 120.0) * 0.0174).sin()) as u8;
            let b = (127.0 + 127.0 * ((hue + 240.0) * 0.0174).sin()) as u8;
            let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);

            settings.cpu_core_colors.push(hex.clone());
            settings_changed = true;
            hex
        };

        cpu_model.push(CpuData {
            usage_str: "0%".into(),
            path_commands: "".into(),
            color: hex_to_color(&color_hex).into(),
        });
    }

    if settings_changed {
        settings.save();
    }

    ui.set_cpus(slint::ModelRc::from(cpu_model.clone()));

    // Apply Settings to UI
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    ui.set_dark_mode(settings.dark_mode);
    ui.set_use_uniform_cpu(settings.use_uniform_cpu);
    ui.set_cpu_chart_color(hex_to_color(&settings.cpu_color).into());
    ui.set_ram_chart_color(hex_to_color(&settings.ram_color).into());
    ui.set_gpu_chart_color(hex_to_color(&settings.gpu_color).into());
    ui.set_net_chart_color(hex_to_color(&settings.net_color).into());

    // GPU Setup
    let nvml_result = Nvml::init();
    let mut gpu_count = 0;

    // Check if NVML initialized successfully
    if let Ok(nvml) = &nvml_result {
        if let Ok(count) = nvml.device_count() {
            gpu_count = count as usize;
            info!("Heimdall detected {} GPU(s)", gpu_count);
        }
    } else {
        error!(
            "NVML init failed (no NVIDIA GPU?): {:?}",
            nvml_result.as_ref().err()
        );
    }

    // Models for Slint
    let gpu_compute_model = Rc::new(slint::VecModel::default());
    let gpu_memory_model = Rc::new(slint::VecModel::default());

    // History buffers for GPU
    let mut gpu_compute_history: Vec<Vec<f32>> = vec![vec![0.0; 60]; gpu_count];
    let mut gpu_memory_history: Vec<Vec<f32>> = vec![vec![0.0; 60]; gpu_count];

    // Initialize GPU models
    if let Ok(nvml) = &nvml_result {
        for i in 0..gpu_count {
            if let Ok(device) = nvml.device_by_index(i as u32) {
                let name = device.name().unwrap_or_else(|_| format!("GPU {}", i));

                // Initial placeholders
                gpu_compute_model.push(CpuData {
                    usage_str: format!("{}: 0%", name).into(),
                    path_commands: "".into(),
                    color: slint::Color::from_rgb_u8(200, 50, 200).into(), // Purple for GPU
                });

                gpu_memory_model.push(CpuData {
                    usage_str: format!("{}: 0 / 0 MB", name).into(),
                    path_commands: "".into(),
                    color: slint::Color::from_rgb_u8(50, 200, 200).into(), // Cyan for VRAM
                });
            }
        }
    }

    ui.set_gpu_compute(slint::ModelRc::from(gpu_compute_model.clone()));
    ui.set_gpu_memory(slint::ModelRc::from(gpu_memory_model.clone()));

    // Network Setup
    let mut networks = Networks::new_with_refreshed_list();
    let network_model = Rc::new(slint::VecModel::default());

    // Create initial network entries
    let mut network_history: Vec<Vec<f32>> = Vec::new(); // History for each interface
                                                         // Only track interfaces that are up? For now track all in list.
                                                         // Iterating networks to build model and history map
                                                         // Note: We need a stable mapping. Vector index might shift if standard iteration order changes?
                                                         // sysinfo Networks iteration is arbitrary? Usually stable by name.

    // Sort keys to be stable
    let mut interface_names: Vec<String> = networks.iter().map(|(n, _)| n.clone()).collect();
    interface_names.sort();

    for (i, name) in interface_names.iter().enumerate() {
        let color = slint::Color::from_rgb_u8(
            (100 + (i * 50) % 155) as u8,
            (150 + (i * 30) % 100) as u8,
            255,
        );
        network_model.push(CpuData {
            usage_str: format!("{}: 0 KB/s", name).into(),
            path_commands: "".into(),
            color: color.into(),
        });
        network_history.push(vec![0.0; 60]);
    }
    ui.set_networks(slint::ModelRc::from(network_model.clone()));

    // Initial clear for memory
    ui.set_memory_path(generate_path(&memory_history, 100.0));

    ui.on_quit(move || {
        slint::quit_event_loop().unwrap();
    });

    let ui_handle = ui.as_weak();
    let save_handle = ui_handle.clone();
    ui.on_save_prefs(move || {
        let ui = save_handle.unwrap();
        // Reload existing to keep hidden state (like per-core colors)
        let mut current_settings = AppSettings::load();

        current_settings.dark_mode = ui.get_dark_mode();
        current_settings.use_uniform_cpu = ui.get_use_uniform_cpu();
        current_settings.cpu_color = brush_to_hex(ui.get_cpu_chart_color());
        current_settings.ram_color = brush_to_hex(ui.get_ram_chart_color());
        current_settings.gpu_color = brush_to_hex(ui.get_gpu_chart_color());
        current_settings.net_color = brush_to_hex(ui.get_net_chart_color());

        current_settings.save();
        info!("Settings saved");
    });

    let timer = Timer::default();
    timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_millis(1000),
        move || {
            let ui = ui_handle.unwrap();

            system.refresh_cpu_all();
            system.refresh_memory();

            // Update each CPU
            for (i, cpu) in system.cpus().iter().enumerate() {
                let usage = cpu.cpu_usage();
                if i < cpus_history.len() {
                    cpus_history[i].remove(0);
                    cpus_history[i].push(usage);

                    let path = generate_path(&cpus_history[i], 100.0);

                    // Update specific row in model, preserving color
                    let mut data = cpu_model.row_data(i).unwrap();
                    data.usage_str = format!("{:.1}%", usage).into();
                    data.path_commands = path;
                    cpu_model.set_row_data(i, data);
                }
            }

            // Update Memory history
            let used_memory = system.used_memory();
            let total_memory = system.total_memory();
            let memory_percentage = (used_memory as f32 / total_memory as f32) * 100.0;

            memory_history.remove(0);
            memory_history.push(memory_percentage);
            ui.set_memory_path(generate_path(&memory_history, 100.0));

            // Update labels
            let used_mem = used_memory as f32 / 1024.0 / 1024.0 / 1024.0; // GB
            let total_mem = total_memory as f32 / 1024.0 / 1024.0 / 1024.0; // GB
            ui.set_memory_label(format!("{:.1} / {:.1} GB", used_mem, total_mem).into());

            // --- GPU Update ---
            if let Ok(nvml) = &nvml_result {
                for i in 0..gpu_count {
                    if let Ok(device) = nvml.device_by_index(i as u32) {
                        let name = device.name().unwrap_or_else(|_| format!("GPU {}", i));

                        // Compute Load
                        let util = device.utilization_rates();
                        let gpu_util = util.map(|u| u.gpu as f32).unwrap_or(0.0);

                        if i < gpu_compute_history.len() {
                            gpu_compute_history[i].remove(0);
                            gpu_compute_history[i].push(gpu_util);

                            let path = generate_path(&gpu_compute_history[i], 100.0);

                            let mut data = gpu_compute_model.row_data(i).unwrap();
                            data.usage_str = format!("{}: {:.0}%", name, gpu_util).into();
                            data.path_commands = path;
                            gpu_compute_model.set_row_data(i, data);
                        }

                        // VRAM
                        let mem = device.memory_info();
                        let (used, total) = match mem {
                            Ok(m) => (
                                m.used as f32 / 1024.0 / 1024.0,
                                m.total as f32 / 1024.0 / 1024.0,
                            ),
                            Err(_) => (0.0, 0.0),
                        };

                        // Calculate percentage for chart? Or just absolute?
                        // Let's chart percentage for consistent 0-100 scaling.
                        let mem_pct = if total > 0.0 {
                            (used / total) * 100.0
                        } else {
                            0.0
                        };

                        if i < gpu_memory_history.len() {
                            gpu_memory_history[i].remove(0);
                            gpu_memory_history[i].push(mem_pct);

                            let path = generate_path(&gpu_memory_history[i], 100.0);

                            let mut data = gpu_memory_model.row_data(i).unwrap();
                            data.usage_str =
                                format!("{}: {:.0} / {:.0} MB", name, used, total).into();
                            data.path_commands = path;
                            gpu_memory_model.set_row_data(i, data);
                        }
                    }
                }
            }

            // --- Network Update ---
            networks.refresh(true);

            // Get default interface for internet detection
            let default_intf_name = default_net::get_default_interface().ok().map(|i| i.name);

            // iterate using stable sorted names
            for (i, name) in interface_names.iter().enumerate() {
                if let Some(net_data) = networks.get(name) {
                    let rx = net_data.received();
                    let tx = net_data.transmitted();
                    let total_rx = net_data.total_received();
                    let total_tx = net_data.total_transmitted();

                    let rx_mb = rx as f32 / 1024.0 / 1024.0;

                    if i < network_history.len() {
                        network_history[i].remove(0);
                        network_history[i].push(rx_mb);

                        let max_val = network_history[i]
                            .iter()
                            .cloned()
                            .fold(0.0 / 0.0, f32::max)
                            .max(1.0);

                        let path = generate_path(&network_history[i], max_val);

                        // Format Rates
                        let fmt_rate = |val: u64| -> String {
                            if val > 1024 * 1024 {
                                format!("{:.1} MB/s", val as f32 / 1024.0 / 1024.0)
                            } else {
                                format!("{:.0} KB/s", val as f32 / 1024.0)
                            }
                        };

                        // Format Totals
                        let fmt_total = |val: u64| -> String {
                            if val > 1024 * 1024 * 1024 {
                                format!("{:.1} GB", val as f32 / 1024.0 / 1024.0 / 1024.0)
                            } else {
                                format!("{:.0} MB", val as f32 / 1024.0 / 1024.0)
                            }
                        };

                        let rx_txt = fmt_rate(rx);
                        let tx_txt = fmt_rate(tx);
                        let total_rx_txt = fmt_total(total_rx);
                        let total_tx_txt = fmt_total(total_tx);

                        // Get IPs
                        let mut ipv4s = Vec::new();
                        let mut ipv6s = Vec::new();

                        for ip in net_data.ip_networks() {
                            match ip.addr {
                                std::net::IpAddr::V4(addr) => ipv4s.push(addr.to_string()),
                                std::net::IpAddr::V6(addr) => ipv6s.push(addr.to_string()),
                            }
                        }

                        // Internet/Gateway indicator
                        let is_gateway = default_intf_name.as_ref().map_or(false, |n| n == name);
                        let gw_icon = if is_gateway { "🌐 " } else { "" };

                        let mut lines = Vec::new();
                        lines.push(format!("{}{}", gw_icon, name));

                        if !ipv4s.is_empty() {
                            lines.push(format!("IPv4: {}", ipv4s.join(", ")));
                        }
                        if !ipv6s.is_empty() {
                            lines.push(format!("IPv6: {}", ipv6s.join(", ")));
                        }

                        lines.push(format!("⬇{} ⬆{}", rx_txt, tx_txt));
                        lines.push(format!("TOT: ⬇{} ⬆{}", total_rx_txt, total_tx_txt));

                        let usage_str = lines.join("\n");

                        let mut data = network_model.row_data(i).unwrap();
                        data.usage_str = usage_str.into();
                        data.path_commands = path;
                        network_model.set_row_data(i, data);
                    }
                }
            }
        },
    );

    ui.run()
}
