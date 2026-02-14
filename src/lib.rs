//! # Gjallarhorn Library
//!
//! This library contains the core logic for the Gjallarhorn resource monitor.

use log::info;
use slint::{Model, Timer, TimerMode};
use std::rc::Rc;

pub mod monitor;
pub mod settings;
pub mod utils;
pub mod worker;

use std::cell::RefCell;

use monitor::SystemMonitor;
use settings::AppSettings;
use utils::{brush_to_hex, generate_path, hex_to_color};

include!(env!("SLINT_INCLUDE_GENERATED"));

/// Runs the Gjallarhorn application.
///
/// This is the main entry point which:
/// 1. Initializes the `SystemMonitor` to gather resource data.
/// 2. Loads persistent `AppSettings` from disk.
/// 3. Sets up the Slint UI (`AppWindow`).
/// 4. Configures the update timer based on the user's refresh rate preference.
/// 5. Binds UI callbacks for saving preferences and quitting.
///
/// Use `cargo run --release` for optimal performance.
pub fn run() -> Result<(), slint::PlatformError> {
    // Initialize logger
    #[cfg(debug_assertions)]
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Error)
        .init();

    let ui = AppWindow::new()?;

    // Load Settings
    let mut settings = AppSettings::load();

    // Initialize Monitor
    let monitor = Rc::new(RefCell::new(SystemMonitor::new(settings.refresh_rate_ms)));
    info!(
        "Gjallarhorn initialized with {} CPUs",
        monitor.borrow().get_cpu_count()
    );

    // --- CPU Model Init ---
    let cpu_model = Rc::new(slint::VecModel::default());
    for i in 0..monitor.borrow().get_cpu_count() {
        // Color management
        let color_hex = if i < settings.cpu_core_colors.len() {
            settings.cpu_core_colors[i].clone()
        } else {
            let hue = (i as f32 * 360.0 / monitor.borrow().get_cpu_count() as f32) % 360.0;
            let r = (127.0 + 127.0 * (hue * 0.0174).sin()) as u8;
            let g = (127.0 + 127.0 * ((hue + 120.0) * 0.0174).sin()) as u8;
            let b = (127.0 + 127.0 * ((hue + 240.0) * 0.0174).sin()) as u8;
            let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);

            settings.cpu_core_colors.push(hex.clone());
            hex
        };

        cpu_model.push(CpuData {
            usage_str: "0%".into(),
            path_commands: "".into(),
            color: hex_to_color(&color_hex).into(),
        });
    }
    settings.save();
    ui.set_cpus(slint::ModelRc::from(cpu_model.clone()));

    // --- GPU Model Init ---
    let gpu_compute_model = Rc::new(slint::VecModel::default());
    let gpu_memory_model = Rc::new(slint::VecModel::default());

    let gpu_data = monitor.borrow().get_gpu_data();
    for data in &gpu_data {
        gpu_compute_model.push(CpuData {
            usage_str: format!("{}: 0%", data.name).into(),
            path_commands: "".into(),
            color: slint::Color::from_rgb_u8(200, 50, 200).into(),
        });
        gpu_memory_model.push(CpuData {
            usage_str: format!("{}: 0 / 0 MB", data.name).into(),
            path_commands: "".into(),
            color: slint::Color::from_rgb_u8(50, 200, 200).into(),
        });
    }
    ui.set_gpu_compute(slint::ModelRc::from(gpu_compute_model.clone()));
    ui.set_gpu_memory(slint::ModelRc::from(gpu_memory_model.clone()));

    // --- Network Model Init ---
    let network_model = Rc::new(slint::VecModel::default());
    let net_data = monitor.borrow().get_network_data();
    for (i, data) in net_data.iter().enumerate() {
        let color = slint::Color::from_rgb_u8(
            (100 + (i * 50) % 155) as u8,
            (150 + (i * 30) % 100) as u8,
            255,
        );
        network_model.push(CpuData {
            usage_str: format!("{}: 0 KB/s", data.name).into(),
            path_commands: "".into(),
            color: color.into(),
        });
    }
    ui.set_networks(slint::ModelRc::from(network_model.clone()));

    // --- Disk Model Init ---
    let disk_model = Rc::new(slint::VecModel::default());
    ui.set_disks(slint::ModelRc::from(disk_model.clone()));

    // Apply Settings
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    ui.set_dark_mode(settings.dark_mode);
    ui.set_use_uniform_cpu(settings.use_uniform_cpu);
    ui.set_refresh_rate_ms(settings.refresh_rate_ms as f32);
    ui.set_cpu_chart_color(hex_to_color(&settings.cpu_color).into());
    ui.set_ram_chart_color(hex_to_color(&settings.ram_color).into());
    ui.set_gpu_chart_color(hex_to_color(&settings.gpu_color).into());
    ui.set_net_chart_color(hex_to_color(&settings.net_color).into());

    // --- System Info Init ---
    let (
        hostname,
        os,
        kernel,
        cpu,
        cores,
        mem,
        bios,
        storage,
        gpus,
        cpu_freq,
        cpu_arch,
        motherboard,
        boot_mode,
        individual_disks,
    ) = monitor.borrow().get_static_info();
    ui.set_sys_hostname(hostname.into());
    ui.set_sys_os_name(os.into());
    ui.set_sys_kernel(kernel.into());
    ui.set_sys_cpu_brand(cpu.into());
    ui.set_sys_cpu_cores(cores as i32);
    ui.set_sys_total_memory(mem.into());
    ui.set_sys_bios_version(bios.into());
    ui.set_sys_storage(storage.into());
    ui.set_sys_gpu_names(gpus.into());
    ui.set_sys_cpu_freq(cpu_freq.into());
    ui.set_sys_cpu_arch(cpu_arch.into());
    ui.set_sys_motherboard(motherboard.into());
    ui.set_sys_boot_mode(boot_mode.into());
    ui.set_sys_disks(individual_disks.into());

    // Detailed Hardware Info
    let cpu_details = monitor.borrow().get_cpu_detailed_info();
    ui.set_sys_cpu_detailed_info(CpuDetailedInfo {
        name: cpu_details.name.into(),
        vendor: cpu_details.vendor.into(),
        architecture: cpu_details.architecture.into(),
        cores_physical: cpu_details.cores_physical as i32,
        cores_logical: cpu_details.cores_logical as i32,
        frequency_current: cpu_details.frequency_current,
        frequency_max: cpu_details.frequency_max,
        frequency_min: cpu_details.frequency_min,
        cache_l1d: cpu_details.cache_l1d.into(),
        cache_l1i: cpu_details.cache_l1i.into(),
        cache_l2: cpu_details.cache_l2.into(),
        cache_l3: cpu_details.cache_l3.into(),
        virtualization: cpu_details.virtualization.into(),
        flags: cpu_details.flags.into(),
    });

    // Detailed Memory Info
    let mem_details = monitor.borrow_mut().get_memory_detailed_info();
    ui.set_sys_memory_detailed_info(MemoryDetailedInfo {
        total_capacity: mem_details.total_capacity.into(),
        used_capacity: mem_details.used_capacity.into(),
        memory_type: mem_details.memory_type.into(),
        speed: mem_details.speed.into(),
        channels: mem_details.channels as i32,
        module_count: mem_details.module_count as i32,
    });

    // Detailed Storage Info
    let storage_details = monitor.borrow().get_storage_detailed_info();
    let storage_details_slint: Vec<StorageDetailedInfo> = storage_details
        .into_iter()
        .map(|d| StorageDetailedInfo {
            device_name: d.device_name.into(),
            model: d.model.into(),
            capacity: format!("{:.2} GB", d.capacity_bytes as f64 / 1_073_741_824.0).into(),
            interface_type: d.interface_type.into(),
            is_ssd: d.is_ssd,
            serial_number: d.serial_number.into(),
            firmware_version: d.firmware_version.into(),
            health_status: d.health_status.into(),
        })
        .collect();
    ui.set_sys_storage_detailed_info(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(storage_details_slint),
    )));

    // Detailed GPU Info
    let gpu_details = monitor.borrow().get_gpu_detailed_info();
    let gpu_details_slint: Vec<GpuDetailedInfo> = gpu_details
        .into_iter()
        .map(|d| GpuDetailedInfo {
            name: d.name.into(),
            vram_total: format!("{:.1} GB", d.vram_total as f64 / 1024.0 / 1024.0 / 1024.0).into(),
            vram_used: format!("{:.1} GB", d.vram_used as f64 / 1024.0 / 1024.0 / 1024.0).into(),
            driver_version: d.driver_version.into(),
            temperature: d
                .temperature
                .map(|t| format!("{}°C", t))
                .unwrap_or("N/A".to_string())
                .into(),
            power_draw: d
                .power_draw
                .map(|p| format!("{:.2} W", p as f64 / 1000.0))
                .unwrap_or("N/A".to_string())
                .into(), // NVML usually returns mW
            power_limit: d
                .power_limit
                .map(|p| format!("{:.2} W", p as f64 / 1000.0))
                .unwrap_or("N/A".to_string())
                .into(),
            fan_speed: d
                .fan_speed
                .map(|f| format!("{}%", f))
                .unwrap_or("N/A".to_string())
                .into(),
            gpu_utilization: d
                .gpu_utilization
                .map(|u| format!("{}%", u))
                .unwrap_or("N/A".to_string())
                .into(),
            memory_utilization: d
                .memory_utilization
                .map(|u| format!("{}%", u))
                .unwrap_or("N/A".to_string())
                .into(),
        })
        .collect();
    ui.set_sys_gpu_detailed_info(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(gpu_details_slint),
    )));

    // Detailed Network Info
    let net_details = monitor.borrow().get_network_detailed_info();
    let net_details_slint: Vec<NetworkDetailedInfo> = net_details
        .into_iter()
        .map(|d| NetworkDetailedInfo {
            name: d.name.into(),
            mac_address: d.mac_address.into(),
            rx_bytes: format!("{:.2} MB", d.rx_bytes as f64 / 1_048_576.0).into(),
            tx_bytes: format!("{:.2} MB", d.tx_bytes as f64 / 1_048_576.0).into(),
            rx_packets: d.rx_packets.to_string().into(),
            tx_packets: d.tx_packets.to_string().into(),
            ip_v4: d.ip_v4.into(),
            ip_v6: d.ip_v6.into(),
            link_speed: d.link_speed.into(),
        })
        .collect();
    ui.set_sys_network_detailed_info(slint::ModelRc::from(std::rc::Rc::new(
        slint::VecModel::from(net_details_slint),
    )));

    // Callbacks
    ui.on_quit(move || {
        slint::quit_event_loop().unwrap();
    });

    let ui_handle = ui.as_weak();

    // --- Timer Logic ---
    let timer = Rc::new(Timer::default());

    // State captured by tick closure
    let tick_monitor = monitor.clone();
    let tick_ui = ui_handle.clone();
    let tick_cpu_model = cpu_model.clone();
    let tick_gpu_comp = gpu_compute_model.clone();
    let tick_gpu_mem = gpu_memory_model.clone();
    let tick_net = network_model.clone();
    let tick_disk = disk_model.clone();

    // Reusable tick closure
    let tick = Rc::new(move || {
        let ui = tick_ui.unwrap();
        let mut monitor = tick_monitor.borrow_mut();

        monitor.refresh();

        // --- Update CPU ---
        for i in 0..monitor.get_cpu_count() {
            if i >= tick_cpu_model.row_count() {
                continue;
            }

            let hist = monitor.get_cpu_history(i);
            if let Some(usage) = hist.back() {
                let mut data = tick_cpu_model.row_data(i).unwrap();
                data.usage_str = format!("{:.1}%", usage).into();
                data.path_commands = generate_path(hist, 100.0, monitor.max_history);
                tick_cpu_model.set_row_data(i, data);
            }
        }

        // --- Update Memory ---
        let (used_gb, total_gb) = monitor.get_memory_info();
        ui.set_memory_label(format!("{:.1} / {:.1} GB", used_gb, total_gb).into());
        ui.set_memory_path(generate_path(
            monitor.get_memory_history(),
            100.0,
            monitor.max_history,
        ));

        // --- Update GPU ---
        let gpu_data = monitor.get_gpu_data();
        for (i, g) in gpu_data.iter().enumerate() {
            if i < tick_gpu_comp.row_count() {
                let mut data = tick_gpu_comp.row_data(i).unwrap();
                data.usage_str = format!("{}: {:.0}%", g.name, g.util).into();
                data.path_commands = generate_path(&g.util_history, 100.0, monitor.max_history);
                tick_gpu_comp.set_row_data(i, data);
            }
            if i < tick_gpu_mem.row_count() {
                let mut data = tick_gpu_mem.row_data(i).unwrap();
                data.usage_str = format!(
                    "{}: {:.0} / {:.0} MB",
                    g.name, g.mem_used_mb, g.mem_total_mb
                )
                .into();
                data.path_commands = generate_path(&g.mem_history, 100.0, monitor.max_history);
                tick_gpu_mem.set_row_data(i, data);
            }
        }

        // --- Update Network ---
        let net_data = monitor.get_network_data();
        for (i, net) in net_data.iter().enumerate() {
            if i < tick_net.row_count() {
                // Formatting
                let fmt_rate = |val: u64| -> String {
                    if val > 1024 * 1024 {
                        format!("{:.1} MB/s", val as f32 / 1024.0 / 1024.0)
                    } else {
                        format!("{:.0} KB/s", val as f32 / 1024.0)
                    }
                };
                let fmt_total = |val: u64| -> String {
                    if val > 1024 * 1024 * 1024 {
                        format!("{:.1} GB", val as f32 / 1024.0 / 1024.0 / 1024.0)
                    } else {
                        format!("{:.0} MB", val as f32 / 1024.0 / 1024.0)
                    }
                };

                let gw_icon = if net.is_default { "🌐 " } else { "" };

                let mut lines = Vec::new();
                lines.push(format!("{}{}", gw_icon, net.name));
                if !net.ips_v4.is_empty() {
                    lines.push(format!("IPv4: {}", net.ips_v4.join(", ")));
                }

                lines.push(format!(
                    "⬇{} ⬆{}",
                    fmt_rate(net.rx_bytes),
                    fmt_rate(net.tx_bytes)
                ));
                lines.push(format!(
                    "TOT: ⬇{} ⬆{}",
                    fmt_total(net.total_rx_bytes),
                    fmt_total(net.total_tx_bytes)
                ));

                let max_val = net.history.iter().fold(f32::NAN, |a, &b| a.max(b)).max(1.0);

                let mut data = tick_net.row_data(i).unwrap();
                data.usage_str = lines.join("\n").into();
                data.path_commands = generate_path(&net.history, max_val, monitor.max_history);
                tick_net.set_row_data(i, data);
            }
        }

        // --- Update Disk ---
        let disks = monitor.get_disk_data();
        if disks.len() != tick_disk.row_count() {
            // Rebuild
            let vec_data: Vec<DiskData> = disks
                .iter()
                .map(|d| {
                    let total_gb = d.total_space_bytes as f32 / 1024.0 / 1024.0 / 1024.0;
                    let used_gb = (d.total_space_bytes - d.available_space_bytes) as f32
                        / 1024.0
                        / 1024.0
                        / 1024.0;
                    let factor = if d.total_space_bytes > 0 {
                        used_gb / total_gb
                    } else {
                        0.0
                    };

                    let bar_color = if factor > 0.9 {
                        slint::Color::from_rgb_u8(231, 76, 60) // Red
                    } else if factor > 0.75 {
                        slint::Color::from_rgb_u8(241, 196, 15) // Yellow
                    } else {
                        slint::Color::from_rgb_u8(46, 204, 113) // Green
                    };

                    DiskData {
                        name: d.name.clone().into(),
                        mount_point: d.mount_point.clone().into(),
                        total: format!("{:.1} GB", total_gb).into(),
                        used: format!("{:.1} GB", used_gb).into(),
                        usage_factor: factor,
                        bar_color: bar_color.into(),
                    }
                })
                .collect();
            tick_disk.set_vec(vec_data);
        } else {
            // Update in place
            for (i, d) in disks.iter().enumerate() {
                let total_gb = d.total_space_bytes as f32 / 1024.0 / 1024.0 / 1024.0;
                let used_gb = (d.total_space_bytes - d.available_space_bytes) as f32
                    / 1024.0
                    / 1024.0
                    / 1024.0;
                let factor = if d.total_space_bytes > 0 {
                    used_gb / total_gb
                } else {
                    0.0
                };

                let bar_color = if factor > 0.9 {
                    slint::Color::from_rgb_u8(231, 76, 60) // Red
                } else if factor > 0.75 {
                    slint::Color::from_rgb_u8(241, 196, 15) // Yellow
                } else {
                    slint::Color::from_rgb_u8(46, 204, 113) // Green
                };

                let mut data = tick_disk.row_data(i).unwrap();
                data.used = format!("{:.1} GB", used_gb).into();
                data.usage_factor = factor;
                data.bar_color = bar_color.into();
                tick_disk.set_row_data(i, data);
            }
        }

        // --- Update Uptime ---
        let uptime_sec = monitor.get_uptime();
        let days = uptime_sec / 86400;
        let hours = (uptime_sec % 86400) / 3600;
        let mins = (uptime_sec % 3600) / 60;
        ui.set_sys_uptime(format!("{}d {}h {}m", days, hours, mins).into());
    });

    // Start Timer
    {
        let t_tick = tick.clone();
        timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(settings.refresh_rate_ms),
            move || t_tick(),
        );
    }

    let save_handle = ui_handle.clone();
    let save_monitor = monitor.clone();
    let save_timer = timer.clone();
    let save_tick = tick.clone();

    ui.on_save_prefs(move || {
        let ui = save_handle.unwrap();
        let mut current_settings = AppSettings::load();

        let old_refresh = current_settings.refresh_rate_ms;

        current_settings.dark_mode = ui.get_dark_mode();
        current_settings.use_uniform_cpu = ui.get_use_uniform_cpu();
        current_settings.refresh_rate_ms = ui.get_refresh_rate_ms() as u64;
        current_settings.cpu_color = brush_to_hex(ui.get_cpu_chart_color());
        current_settings.ram_color = brush_to_hex(ui.get_ram_chart_color());
        current_settings.gpu_color = brush_to_hex(ui.get_gpu_chart_color());
        current_settings.net_color = brush_to_hex(ui.get_net_chart_color());
        current_settings.save();
        info!("Settings saved");

        // Handle refresh rate change
        if current_settings.refresh_rate_ms != old_refresh {
            info!(
                "Updating refresh rate to {}ms",
                current_settings.refresh_rate_ms
            );
            save_monitor
                .borrow_mut()
                .set_refresh_rate(current_settings.refresh_rate_ms);

            // Restart timer
            let t_tick = save_tick.clone();
            save_timer.start(
                TimerMode::Repeated,
                std::time::Duration::from_millis(current_settings.refresh_rate_ms),
                move || t_tick(),
            );
        }
    });

    ui.run()
}
