//! # System Monitor Module
//!
//! This module acts as the central aggregator for all system resource data.
//! It integrates:
//! - `sysinfo` for CPU, Memory, and Disk usage.
//! - `nvml-wrapper` for NVIDIA GPU statistics.
//! - `default-net` (via `sysinfo::Networks`) for Network traffic monitoring.
//!
//! The `SystemMonitor` struct maintains historical data buffers (sliding windows)
//! for each metric to facilitate real-time graph rendering.

use log::{error, info};
use nvml_wrapper::Nvml;
use std::collections::VecDeque;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

/// Holds data for a single CPU core for external consumers
#[allow(dead_code)]
pub struct CoreData {
    pub usage: f32,
    pub history: Vec<f32>,
}

/// Holds data for GPU
pub struct GpuData {
    pub name: String,
    pub util: f32,
    pub mem_used_mb: f32,
    pub mem_total_mb: f32,
    pub util_history: Vec<f32>,
    pub mem_history: Vec<f32>,
}

/// Holds data for Network Interface
pub struct NetworkData {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub history: Vec<f32>, // Stores RX in MB for graph
    pub ips_v4: Vec<String>,
    // pub ips_v6: Vec<String>, // Unused for now
    pub is_default: bool,
}

/// Holds data for Disk
pub struct DiskData {
    pub name: String,
    pub mount_point: String,
    pub total_space_bytes: u64,
    pub available_space_bytes: u64,
    // pub is_removable: bool, // Unused
}

/// The core system monitoring struct.
///
/// It holds the state of the system resources and maintains historical data for rendering graphs.
pub struct SystemMonitor {
    pub system: System,
    pub disks: Disks,
    pub networks: Networks,
    pub nvml: Option<Nvml>,

    /// Sliding window of CPU usage history (per core).
    pub cpu_history: Vec<VecDeque<f32>>,
    /// Sliding window of Memory usage history (percent).
    pub mem_history: VecDeque<f32>,
    /// Sliding window of GPU Utilization history (per GPU).
    pub gpu_util_history: Vec<VecDeque<f32>>,
    /// Sliding window of GPU Memory usage history (per GPU).
    pub gpu_mem_history: Vec<VecDeque<f32>>,
    /// Sliding window of Network RX history (per Interface).
    pub net_history: Vec<VecDeque<f32>>, // Keyed by sorted interface index

    /// Stable sorted interface names to ensure consistent indexing across refreshes.
    pub interface_names: Vec<String>,

    /// Maximum number of data points to keep in history buffers.
    /// Calculated based on refresh rate to maintain a 60-second window.
    pub max_history: usize,
}

impl SystemMonitor {
    /// Creates a new `SystemMonitor` instance.
    ///
    /// Initializes `sysinfo` components, detects NVIDIA GPUs via `nvml`, and pre-allocation
    /// history buffers based on the provided `refresh_rate_ms`.
    ///
    /// # Arguments
    ///
    /// * `refresh_rate_ms` - The update interval in milliseconds. Used to calculate the buffer size for a 60s window.
    pub fn new(refresh_rate_ms: u64) -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        // Initial Refresh
        system.refresh_cpu_all();
        system.refresh_memory();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let mut interface_names: Vec<String> = networks.keys().cloned().collect();
        interface_names.sort();

        let nvml = match Nvml::init() {
            Ok(n) => {
                info!("NVML Initialized");
                Some(n)
            }
            Err(e) => {
                error!("NVML Init failed: {:?}", e);
                None
            }
        };

        let cpu_count = system.cpus().len();
        // 60 seconds * (1000 / ms) updates/second
        let max_history = (60 * 1000 / refresh_rate_ms).max(1) as usize;

        // GPU Count logic
        let gpu_count = if let Some(n) = &nvml {
            n.device_count().unwrap_or(0) as usize
        } else {
            0
        };

        SystemMonitor {
            system,
            disks,
            networks,
            nvml,
            cpu_history: vec![VecDeque::from(vec![0.0; max_history]); cpu_count],
            mem_history: VecDeque::from(vec![0.0; max_history]),
            gpu_util_history: vec![VecDeque::from(vec![0.0; max_history]); gpu_count],
            gpu_mem_history: vec![VecDeque::from(vec![0.0; max_history]); gpu_count],
            net_history: vec![VecDeque::from(vec![0.0; max_history]); interface_names.len()],
            interface_names,
            max_history,
        }
    }

    /// Updates the refresh rate and resizes history buffers accordingly.
    ///
    /// This ensures that the graph history always represents exactly 60 seconds of data,
    /// regardless of how often the data is polled.
    pub fn set_refresh_rate(&mut self, ms: u64) {
        self.max_history = (60 * 1000 / ms).max(1) as usize;

        // Resize buffers
        // CPU
        for h in &mut self.cpu_history {
            h.resize(self.max_history, 0.0);
        }
        // RAM
        self.mem_history.resize(self.max_history, 0.0);

        // GPU
        for h in &mut self.gpu_util_history {
            h.resize(self.max_history, 0.0);
        }
        for h in &mut self.gpu_mem_history {
            h.resize(self.max_history, 0.0);
        }

        // Net
        for h in &mut self.net_history {
            h.resize(self.max_history, 0.0);
        }
    }

    /// Polls the system for current resource usage and updates history buffers.
    ///
    /// This should be called once per tick (timer event).
    pub fn refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(true);

        // --- Update CPU History ---
        // Ensure we have enough buffers if CPU count changed (unlikely but safe)
        if self.system.cpus().len() != self.cpu_history.len() {
            self.cpu_history.resize(
                self.system.cpus().len(),
                VecDeque::from(vec![0.0; self.max_history]),
            );
        }

        for (i, cpu) in self.system.cpus().iter().enumerate() {
            if i < self.cpu_history.len() {
                self.cpu_history[i].pop_front();
                self.cpu_history[i].push_back(cpu.cpu_usage());
            }
        }

        // --- Update Memory History ---
        let used = self.system.used_memory() as f32;
        let total = self.system.total_memory() as f32;
        let pct = if total > 0.0 {
            (used / total) * 100.0
        } else {
            0.0
        };
        self.mem_history.pop_front();
        self.mem_history.push_back(pct);

        // --- Update GPU History ---
        if let Some(nvml) = &self.nvml {
            if let Ok(count) = nvml.device_count() {
                let count = count as usize;
                if count != self.gpu_util_history.len() {
                    // Resize if strictly needed
                    self.gpu_util_history
                        .resize(count, VecDeque::from(vec![0.0; self.max_history]));
                    self.gpu_mem_history
                        .resize(count, VecDeque::from(vec![0.0; self.max_history]));
                }

                for i in 0..count {
                    if let Ok(dev) = nvml.device_by_index(i as u32) {
                        // Util
                        let util = dev.utilization_rates().map(|u| u.gpu as f32).unwrap_or(0.0);
                        self.gpu_util_history[i].pop_front();
                        self.gpu_util_history[i].push_back(util);

                        // Mem
                        let mem_info = dev.memory_info();
                        let mem_pct = match mem_info {
                            Ok(m) if m.total > 0 => (m.used as f32 / m.total as f32) * 100.0,
                            _ => 0.0,
                        };
                        self.gpu_mem_history[i].pop_front();
                        self.gpu_mem_history[i].push_back(mem_pct);
                    }
                }
            }
        }

        // --- Update Network History ---
        // Check if interfaces changed? For now assume valid index mapping via sorted keys
        for (i, name) in self.interface_names.iter().enumerate() {
            if let Some(net) = self.networks.get(name) {
                let rx_mb = net.received() as f32 / 1024.0 / 1024.0;
                if i < self.net_history.len() {
                    self.net_history[i].pop_front();
                    self.net_history[i].push_back(rx_mb);
                }
            }
        }
    }

    pub fn get_cpu_count(&self) -> usize {
        self.system.cpus().len()
    }

    // Helper to get raw history as Vec for UI generation
    pub fn get_cpu_history(&self, index: usize) -> Vec<f32> {
        if index < self.cpu_history.len() {
            self.cpu_history[index].iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_memory_info(&self) -> (f32, f32) {
        let used = self.system.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
        let total = self.system.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
        (used, total)
    }

    pub fn get_memory_history(&self) -> Vec<f32> {
        self.mem_history.iter().cloned().collect()
    }

    pub fn get_gpu_data(&self) -> Vec<GpuData> {
        let mut data = Vec::new();
        if let Some(nvml) = &self.nvml {
            if let Ok(count) = nvml.device_count() {
                for i in 0..count {
                    if let Ok(dev) = nvml.device_by_index(i as u32) {
                        let name = dev.name().unwrap_or(format!("GPU {}", i));
                        let util = self
                            .gpu_util_history
                            .get(i as usize)
                            .and_then(|v| v.back())
                            .cloned()
                            .unwrap_or(0.0);

                        let (mem_used, mem_total) = match dev.memory_info() {
                            Ok(m) => (
                                m.used as f32 / 1024.0 / 1024.0,
                                m.total as f32 / 1024.0 / 1024.0,
                            ),
                            _ => (0.0, 0.0),
                        };

                        data.push(GpuData {
                            name,
                            util,
                            mem_used_mb: mem_used,
                            mem_total_mb: mem_total,
                            util_history: self
                                .gpu_util_history
                                .get(i as usize)
                                .map(|v| v.iter().cloned().collect())
                                .unwrap_or_default(),
                            mem_history: self
                                .gpu_mem_history
                                .get(i as usize)
                                .map(|v| v.iter().cloned().collect())
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }
        data
    }

    pub fn get_network_data(&self) -> Vec<NetworkData> {
        let default_interface = default_net::get_default_interface().ok().map(|i| i.name);

        let mut res = Vec::new();
        for (i, name) in self.interface_names.iter().enumerate() {
            if let Some(net) = self.networks.get(name) {
                let mut ipv4s = Vec::new();
                // let mut ipv6s = Vec::new();
                for ip in net.ip_networks() {
                    match ip.addr {
                        std::net::IpAddr::V4(a) => ipv4s.push(a.to_string()),
                        std::net::IpAddr::V6(_a) => {} // ipv6s.push(a.to_string()),
                    }
                }

                res.push(NetworkData {
                    name: name.clone(),
                    rx_bytes: net.received(),
                    tx_bytes: net.transmitted(),
                    total_rx_bytes: net.total_received(),
                    total_tx_bytes: net.total_transmitted(),
                    history: self
                        .net_history
                        .get(i)
                        .map(|v| v.iter().cloned().collect())
                        .unwrap_or_default(),
                    ips_v4: ipv4s,
                    // ips_v6: ipv6s,
                    is_default: default_interface.as_ref() == Some(name),
                });
            }
        }
        res
    }

    pub fn get_disk_data(&self) -> Vec<DiskData> {
        let mut res = Vec::new();
        for disk in &self.disks {
            res.push(DiskData {
                name: disk.name().to_string_lossy().into_owned(),
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                total_space_bytes: disk.total_space(),
                available_space_bytes: disk.available_space(),
                // is_removable: disk.is_removable(),
            });
        }
        res
    }

    pub fn get_static_info(
        &self,
    ) -> (
        String,
        String,
        String,
        String,
        usize,
        String,
        String,
        String,
        String,
    ) {
        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_ver = System::os_version().unwrap_or_else(|| "".to_string());
        let kernel = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());

        let cpu_brand = self
            .system
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();
        let cores = self.system.cpus().len();

        let total_mem = format!(
            "{:.1} GB",
            self.system.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0
        );

        // BIOS Version
        let bios_version = std::fs::read_to_string("/sys/class/dmi/id/bios_version")
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();

        // Total Storage
        let total_storage_bytes: u64 = self.disks.iter().map(|d| d.total_space()).sum();
        let total_storage = format!(
            "{:.1} GB",
            total_storage_bytes as f32 / 1024.0 / 1024.0 / 1024.0
        );

        // GPU Names
        let mut gpu_names = Vec::new();
        if let Some(nvml) = &self.nvml {
            if let Ok(count) = nvml.device_count() {
                for i in 0..count {
                    if let Ok(dev) = nvml.device_by_index(i as u32) {
                        gpu_names.push(dev.name().unwrap_or_else(|_| format!("NVIDIA GPU {}", i)));
                    }
                }
            }
        }
        let gpu_str = if gpu_names.is_empty() {
            "".to_string()
        } else {
            gpu_names.join(", ")
        };

        (
            hostname,
            format!("{} {}", os_name, os_ver),
            kernel,
            cpu_brand,
            cores,
            total_mem,
            bios_version,
            total_storage,
            gpu_str,
        )
    }

    pub fn get_uptime(&self) -> u64 {
        System::uptime()
    }
}
