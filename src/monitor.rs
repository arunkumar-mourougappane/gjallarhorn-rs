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
use sysinfo::{Disks, Networks, System};

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

// Detailed hardware information structures for sub-tabs
#[derive(Debug, Clone)]
pub struct CpuDetailedInfo {
    pub name: String,
    pub vendor: String,
    pub architecture: String,
    pub cores_physical: usize,
    pub cores_logical: usize,
    pub frequency_current: f32,
    pub frequency_max: f32,
    pub frequency_min: f32,
    pub cache_l1d: String,
    pub cache_l1i: String,
    pub cache_l2: String,
    pub cache_l3: String,
    pub virtualization: String,
    pub flags: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryDetailedInfo {
    pub total_capacity: String,
    pub used_capacity: String,
    pub memory_type: String,
    pub speed: String,
    pub channels: u32,
    pub module_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageDetailedInfo {
    pub device_name: String,
    pub model: String,
    pub capacity_bytes: u64,
    pub interface_type: String,
    pub is_ssd: bool,
    pub serial_number: String,
    pub firmware_version: String,
    pub health_status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuDetailedInfo {
    pub name: String,
    pub vram_total: u64,
    // ... (rest omitted, but replace block needs to be complete or targeted)
    // Wait, I should target specific lines or reuse whole block.
    // I'll use separate replacements for safety if possible? No, multi_replace.
    // I'll target the derive lines.
    pub vram_used: u64,
    pub driver_version: String,
    pub temperature: Option<i32>,
    pub power_draw: Option<f32>,
    pub power_limit: Option<f32>,
    pub fan_speed: Option<u32>,
    pub gpu_utilization: Option<u32>,
    pub memory_utilization: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkDetailedInfo {
    pub name: String,
    pub mac_address: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub ip_v4: String,
    pub ip_v6: String,
    pub link_speed: String,
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

    // Privileged Data (Shared with UI)
    pub privileged_data: std::sync::Arc<std::sync::Mutex<Option<crate::worker::PrivilegedData>>>,
}

impl SystemMonitor {
    /// Creates a new `SystemMonitor` instance.
    ///
    /// Initializes `sysinfo` components, detects NVIDIA GPUs via `nvml`, and pre-allocation
    /// history buffers based on the provided `refresh_rate_ms`.
    /// Also spawns the privileged worker process if possible.
    pub fn new(refresh_rate_ms: u64) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        // Privileged Data Holder
        let privileged_data = std::sync::Arc::new(std::sync::Mutex::new(None));
        let privileged_data_clone = privileged_data.clone();

        // Spawn Worker Thread
        std::thread::spawn(move || {
            let exe = std::env::current_exe().unwrap();
            // Try to spawn worker via pkexec
            // Note: pkexec might prompt for password.
            if let Ok(mut child) = std::process::Command::new("pkexec")
                .arg(exe)
                .arg("--privileged-worker")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null()) // suppress errors or redirect?
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let reader = std::io::BufReader::new(stdout);
                    use std::io::BufRead;
                    for line in reader.lines() {
                        if let Ok(json) = line {
                            if let Ok(data) =
                                serde_json::from_str::<crate::worker::PrivilegedData>(&json)
                            {
                                if let Ok(mut guard) = privileged_data_clone.lock() {
                                    *guard = Some(data);
                                }
                            }
                        }
                    }
                }
                let _ = child.wait();
            } else {
                error!("Failed to spawn privileged worker via pkexec.");
            }
        });

        // Initialize NVML
        let nvml = match Nvml::init() {
            Ok(n) => Some(n),
            Err(e) => {
                error!("NVML Init failed: {}", e);
                None
            }
        };

        let mut interface_names: Vec<String> = networks.keys().cloned().collect();
        interface_names.sort();

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
            privileged_data,
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

    // Helper to get raw history as reference for UI generation
    pub fn get_cpu_history(&self, index: usize) -> &VecDeque<f32> {
        static EMPTY: VecDeque<f32> = VecDeque::new();
        if index < self.cpu_history.len() {
            &self.cpu_history[index]
        } else {
            &EMPTY
        }
    }

    pub fn get_memory_info(&self) -> (f32, f32) {
        let used = self.system.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
        let total = self.system.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
        (used, total)
    }

    pub fn get_memory_history(&self) -> &VecDeque<f32> {
        &self.mem_history
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
                                .map(|v| Vec::from_iter(v.iter().copied()))
                                .unwrap_or_default(),
                            mem_history: self
                                .gpu_mem_history
                                .get(i as usize)
                                .map(|v| Vec::from_iter(v.iter().copied()))
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
                        .map(|v| Vec::from_iter(v.iter().copied()))
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
        String, // hostname
        String, // os_name
        String, // kernel
        String, // cpu_brand
        usize,  // cores
        String, // total_mem
        String, // bios_version
        String, // total_storage
        String, // gpu_str
        String, // cpu_freq
        String, // cpu_arch
        String, // motherboard
        String, // boot_mode
        String, // individual_disks
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

        // GPU Names with VRAM
        let mut gpu_names = Vec::new();
        if let Some(nvml) = &self.nvml {
            if let Ok(count) = nvml.device_count() {
                for i in 0..count {
                    if let Ok(dev) = nvml.device_by_index(i as u32) {
                        let name = dev.name().unwrap_or_else(|_| format!("NVIDIA GPU {}", i));
                        let vram = if let Ok(mem_info) = dev.memory_info() {
                            let vram_gb = mem_info.total as f32 / 1024.0 / 1024.0 / 1024.0;
                            format!(" ({:.0} GB)", vram_gb)
                        } else {
                            String::new()
                        };
                        gpu_names.push(format!("{}{}", name, vram));
                    }
                }
            }
        }
        let gpu_str = if gpu_names.is_empty() {
            "".to_string()
        } else {
            gpu_names.join(", ")
        };

        // CPU Frequency
        let cpu_freq = self
            .system
            .cpus()
            .first()
            .map(|c| format!("{:.2} GHz", c.frequency() as f32 / 1000.0))
            .unwrap_or_else(|| "N/A".to_string());

        // CPU Architecture
        let cpu_arch = std::env::consts::ARCH.to_string();

        // Motherboard Info
        let board_vendor = std::fs::read_to_string("/sys/class/dmi/id/board_vendor")
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();
        let board_name = std::fs::read_to_string("/sys/class/dmi/id/board_name")
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();
        let motherboard = if board_vendor != "Unknown" && board_name != "Unknown" {
            format!("{} {}", board_vendor, board_name)
        } else {
            "Unknown".to_string()
        };

        // Boot Mode (UEFI or Legacy)
        let boot_mode = if std::path::Path::new("/sys/firmware/efi").exists() {
            "UEFI".to_string()
        } else {
            "Legacy BIOS".to_string()
        };

        // Physical Disks (not partitions)
        let physical_disks = Self::get_physical_disks();
        let individual_disks = if physical_disks.is_empty() {
            "None detected".to_string()
        } else {
            physical_disks
                .iter()
                .map(|(name, model, size_bytes)| {
                    let size_gb = *size_bytes as f32 / 1024.0 / 1024.0 / 1024.0;
                    if model.is_empty() || model == "Unknown" {
                        format!("{} ({:.1} GB)", name, size_gb)
                    } else {
                        format!("{} ({:.1} GB)", model, size_gb)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
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
            cpu_freq,
            cpu_arch,
            motherboard,
            boot_mode,
            individual_disks,
        )
    }

    /// Get physical disk information (models, not partitions)
    fn get_physical_disks() -> Vec<(String, String, u64)> {
        let mut disks = Vec::new();

        // Read /sys/class/block/ for block devices
        if let Ok(entries) = std::fs::read_dir("/sys/class/block") {
            for entry in entries.flatten() {
                let device_name = entry.file_name().to_string_lossy().to_string();

                // Filter: only base devices (nvme0n1, sda), not partitions (nvme0n1p1, sda1)
                // NVMe: nvme0n1, nvme1n1 (not nvme0n1p1)
                // SATA/SAS: sda, sdb, sdc (not sda1)
                // Virtual: vda, vdb (not vda1)
                let is_partition = if device_name.starts_with("nvme") {
                    // nvme0n1p1 is partition, nvme0n1 is not
                    device_name.contains('p')
                        && device_name
                            .chars()
                            .last()
                            .map_or(false, |c| c.is_ascii_digit())
                } else if device_name.starts_with("sd") || device_name.starts_with("vd") {
                    // sda1, vda1 are partitions, sda, vda are not
                    device_name
                        .chars()
                        .last()
                        .map_or(false, |c| c.is_ascii_digit())
                } else {
                    // Skip loop devices, ram, zram, etc.
                    continue;
                };

                if is_partition {
                    continue;
                }

                // Read device model
                let model_path = format!("/sys/class/block/{}/device/model", device_name);
                let mut model = std::fs::read_to_string(&model_path)
                    .unwrap_or_else(|_| "Unknown".to_string())
                    .trim()
                    .to_string();

                // For NVMe, try alternative path
                if model == "Unknown" && device_name.starts_with("nvme") {
                    let nvme_model_path = format!("/sys/class/block/{}/device/model", device_name);
                    model = std::fs::read_to_string(&nvme_model_path)
                        .unwrap_or_else(|_| "Unknown".to_string())
                        .trim()
                        .to_string();
                }

                // Read device size (in 512-byte sectors)
                let size_path = format!("/sys/class/block/{}/size", device_name);
                let size_sectors: u64 = std::fs::read_to_string(&size_path)
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let size_bytes = size_sectors * 512;

                // Only add if size > 0 (exclude empty devices)
                if size_bytes > 0 {
                    disks.push((device_name, model, size_bytes));
                }
            }
        }

        disks.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by device name
        disks
    }

    pub fn get_uptime(&self) -> u64 {
        System::uptime()
    }

    /// Get detailed CPU information
    pub fn get_cpu_detailed_info(&self) -> CpuDetailedInfo {
        // Read /proc/cpuinfo for detailed CPU data
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

        // Parse vendor_id
        let vendor = cpuinfo
            .lines()
            .find(|line| line.starts_with("vendor_id"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Parse model name
        let name = cpuinfo
            .lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown Processor".to_string());

        // Parse physical cores
        let cores_physical = cpuinfo
            .lines()
            .find(|line| line.starts_with("cpu cores"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(self.system.cpus().len());

        // Parse cache size (L3 cache typically listed as "cache size")
        let cache_size_kb = cpuinfo
            .lines()
            .find(|line| line.starts_with("cache size"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().split_whitespace().next())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        // Parse flags for capabilities
        let flags_line = cpuinfo
            .lines()
            .find(|line| line.starts_with("flags"))
            .map(|line| line.to_string())
            .unwrap_or_default();

        // Check for virtualization support
        let virtualization = if flags_line.contains("vmx") {
            "VT-x (Intel)".to_string()
        } else if flags_line.contains("svm") {
            "AMD-V (AMD)".to_string()
        } else {
            "Not detected".to_string()
        };

        // Extract important instruction sets
        let mut important_flags = Vec::new();
        for flag in &["sse4_2", "avx", "avx2", "avx512f", "aes", "sha_ni"] {
            if flags_line.contains(flag) {
                important_flags.push(flag.to_uppercase());
            }
        }
        let flags = if important_flags.is_empty() {
            "Standard".to_string()
        } else {
            important_flags.join(", ")
        };

        // Get frequency info from sysinfo
        let frequency_current = self
            .system
            .cpus()
            .first()
            .map(|cpu| cpu.frequency() as f32 / 1000.0)
            .unwrap_or(0.0);

        // Try to read max/min frequency from sysfs
        let frequency_max =
            std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|f| f / 1_000_000.0) // Convert kHz to GHz
                .unwrap_or(0.0);

        let frequency_min =
            std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq")
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|f| f / 1_000_000.0)
                .unwrap_or(0.0);

        // Parse cache information from lscpu or sysfs
        let cache_l3 = if cache_size_kb > 0 {
            format!("{} KB", cache_size_kb)
        } else {
            "N/A".to_string()
        };

        // Try to get L1/L2 cache from sysfs
        let cache_l1d = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index0/size")
            .unwrap_or_else(|_| "N/A".to_string())
            .trim()
            .to_string();

        let cache_l1i = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index1/size")
            .unwrap_or_else(|_| "N/A".to_string())
            .trim()
            .to_string();

        let cache_l2 = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index2/size")
            .unwrap_or_else(|_| "N/A".to_string())
            .trim()
            .to_string();

        CpuDetailedInfo {
            name,
            vendor,
            architecture: std::env::consts::ARCH.to_string(),
            cores_physical,
            cores_logical: self.system.cpus().len(),
            frequency_current,
            frequency_max,
            frequency_min,
            cache_l1d,
            cache_l1i,
            cache_l2,
            cache_l3,
            virtualization,
            flags,
        }
    }

    /// Get detailed memory information
    pub fn get_memory_detailed_info(&mut self) -> MemoryDetailedInfo {
        // Basic info from sysinfo
        self.system.refresh_memory();
        let total_mem = self.system.total_memory();
        let used_mem = self.system.used_memory();
        let total_capacity = format!("{:.1} GB", total_mem as f64 / 1024.0 / 1024.0 / 1024.0);
        let used_capacity = format!("{:.1} GB", used_mem as f64 / 1024.0 / 1024.0 / 1024.0);

        // Detailed info from dmidecode
        let mut memory_type = "Unknown".to_string();
        let mut speed = "Unknown".to_string();
        let mut module_count = 0;
        let channels;

        // Try dmidecode
        if let Ok(output) = std::process::Command::new("dmidecode")
            .arg("-t")
            .arg("memory")
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let devices: Vec<&str> = stdout.split("Memory Device").collect();
                // Skip the first split part as it's header/preamble
                for device in devices.iter().skip(1) {
                    // Check if device is present (Size is not "No Module Installed")
                    if device.contains("Size: No Module Installed") {
                        continue;
                    }

                    // Extract Type
                    if memory_type == "Unknown" {
                        if let Some(line) = device.lines().find(|l| l.trim().starts_with("Type:")) {
                            memory_type = line.split(':').nth(1).unwrap_or("").trim().to_string();
                        }
                    }

                    // Extract Speed
                    if speed == "Unknown" {
                        if let Some(line) = device.lines().find(|l| l.trim().starts_with("Speed:"))
                        {
                            let s = line.split(':').nth(1).unwrap_or("").trim();
                            if s != "Unknown" {
                                speed = s.to_string();
                            }
                        }
                    }
                    module_count += 1;
                }
            } else {
                memory_type = "Root required".to_string();
                speed = "Root required".to_string();
            }
        } else {
            // dmidecode not found or failed to run
            memory_type = "Unknown".to_string();
            speed = "Unknown".to_string();
        }

        channels = module_count;

        MemoryDetailedInfo {
            total_capacity,
            used_capacity,
            memory_type,
            speed,
            channels,
            module_count,
        }
    }

    /// Get detailed storage information for all physical disks
    pub fn get_storage_detailed_info(&self) -> Vec<StorageDetailedInfo> {
        // Try to get privileged data first
        if let Ok(guard) = self.privileged_data.lock() {
            if let Some(data) = &*guard {
                if !data.storage.is_empty() {
                    return data.storage.clone();
                }
            }
        }

        // Fallback to non-privileged gathering (or repetitive legacy logic)
        // Since we extracted the headless logic, we can just call it?
        // Wait, headless logic uses `sysinfo` or just `/sys/class/block`?
        // `get_storage_detailed_info_headless` is static and does not use `self`.
        // So we can just call it! It works for both.
        // But wait, the "Legacy" logic inside `Monitor` had `self`? No, it just iterated `/sys`.
        // So I can replace the entire body with:

        crate::monitor::get_storage_detailed_info_headless()
    }

    /// Get detailed GPU information
    pub fn get_gpu_detailed_info(&self) -> Vec<GpuDetailedInfo> {
        let mut gpus = Vec::new();

        if let Some(nvml) = &self.nvml {
            if let Ok(count) = nvml.device_count() {
                for i in 0..count {
                    if let Ok(dev) = nvml.device_by_index(i as u32) {
                        let name = dev.name().unwrap_or_else(|_| format!("NVIDIA GPU {}", i));

                        // Memory info
                        let (vram_total, vram_used) = dev
                            .memory_info()
                            .map(|mem| (mem.total, mem.used))
                            .unwrap_or((0, 0));

                        // Driver version
                        let driver_version = nvml
                            .sys_driver_version()
                            .unwrap_or_else(|_| "Unknown".to_string());

                        // Temperature
                        let temperature = dev
                            .temperature(
                                nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu,
                            )
                            .ok()
                            .map(|t| t as i32);

                        // Power
                        let power_draw = dev.power_usage().ok().map(|p| p as f32 / 1000.0); // Convert mW to W

                        let power_limit =
                            dev.power_management_limit().ok().map(|p| p as f32 / 1000.0);

                        // Fan speed
                        let fan_speed = dev.fan_speed(0).ok();

                        // Utilization
                        let gpu_utilization = dev.utilization_rates().ok().map(|u| u.gpu);

                        let memory_utilization = dev.utilization_rates().ok().map(|u| u.memory);

                        gpus.push(GpuDetailedInfo {
                            name,
                            vram_total,
                            vram_used,
                            driver_version,
                            temperature,
                            power_draw,
                            power_limit,
                            fan_speed,
                            gpu_utilization,
                            memory_utilization,
                        });
                    }
                }
            }
        }

        gpus
    }

    /// Get detailed network information
    pub fn get_network_detailed_info(&self) -> Vec<NetworkDetailedInfo> {
        // Try to get privileged data first
        if let Ok(guard) = self.privileged_data.lock() {
            if let Some(data) = &*guard {
                if !data.network.is_empty() {
                    return data.network.clone();
                }
            }
        }

        // Fallback
        crate::monitor::get_network_detailed_info_headless(&self.networks)
    }
}
// --- Standalone Data Gathering Functions (Reused by Worker) ---

pub fn get_storage_detailed_info_headless() -> Vec<StorageDetailedInfo> {
    let mut storage_devices = Vec::new();
    // Read /sys/class/block for devices
    let entries = match std::fs::read_dir("/sys/class/block") {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let device_name = entry.file_name().to_string_lossy().to_string();

        // Filter: only physical disk-like devices (sd*, nvme*n1), exclude partitions (sd*1) and loop
        if device_name.starts_with("loop")
            || device_name.starts_with("ram")
            || device_name.starts_with("sr")
        {
            continue;
        }
        // Exclude partitions: check if it ends with digit (for sd*) or p+digit (nvme)
        // Heuristic: check if /sys/class/block/{name}/partition exists
        let partition_path = format!("/sys/class/block/{}/partition", device_name);
        if std::path::Path::new(&partition_path).exists() {
            continue;
        }

        // Capacity
        let size_path = format!("/sys/class/block/{}/size", device_name);
        let capacity_sectors = std::fs::read_to_string(&size_path)
            .unwrap_or("0".to_string())
            .trim()
            .parse::<u64>()
            .unwrap_or(0);
        let capacity_bytes = capacity_sectors * 512; // Standard sector size assumption

        // Model
        let model_path = format!("/sys/class/block/{}/device/model", device_name);
        let mut model = std::fs::read_to_string(&model_path)
            .unwrap_or("Unknown".to_string())
            .trim()
            .to_string();

        if model == "Unknown" && device_name.starts_with("nvme") {
            // NVMe model path
            if let Ok(m) =
                std::fs::read_to_string(format!("/sys/class/block/{}/device/model", device_name))
            {
                model = m.trim().to_string();
            }
        }

        // Interface Type
        let interface_type = if device_name.starts_with("nvme") {
            "NVMe".to_string()
        } else if device_name.starts_with("sd") {
            "SATA".to_string()
        } else if device_name.starts_with("vd") {
            "VirtIO".to_string()
        } else {
            "Unknown".to_string()
        };

        // SSD Check
        let rotational_path = format!("/sys/class/block/{}/queue/rotational", device_name);
        let is_ssd = std::fs::read_to_string(&rotational_path)
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .map(|v| v == 0)
            .unwrap_or(true);

        // Serial & Firmware (Fallback)
        let mut serial_number =
            std::fs::read_to_string(format!("/sys/class/block/{}/device/serial", device_name))
                .unwrap_or("Unknown".to_string())
                .trim()
                .to_string();
        let mut firmware_version =
            std::fs::read_to_string(format!("/sys/class/block/{}/device/rev", device_name))
                .unwrap_or("Unknown".to_string())
                .trim()
                .to_string();

        if device_name.starts_with("nvme") && firmware_version == "Unknown" {
            if let Ok(fw) = std::fs::read_to_string(format!(
                "/sys/class/block/{}/device/firmware_rev",
                device_name
            )) {
                firmware_version = fw.trim().to_string();
            }
        }

        // Health via smartctl (Privileged part)
        let mut health_status = "Unknown".to_string();

        // Only try smartctl if we are likely root (headless fn implies usage by worker) or it's installed
        // The worker will be root, so this should succeed.
        if let Ok(output) = std::process::Command::new("smartctl")
            .args(&["--json", "-a", &format!("/dev/{}", device_name)])
            .output()
        {
            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(s) = v["serial_number"].as_str() {
                        serial_number = s.to_string();
                    }
                    if let Some(f) = v["firmware_version"].as_str() {
                        firmware_version = f.to_string();
                    }
                    if let Some(passed) = v["smart_status"]["passed"].as_bool() {
                        health_status = if passed {
                            "Passed".to_string()
                        } else {
                            "Failed".to_string()
                        };
                    }
                    if health_status == "Unknown" {
                        if let Some(nvme_health) =
                            v["nvme_smart_health_information_log"]["critical_warning"].as_u64()
                        {
                            health_status = if nvme_health == 0 {
                                "Passed".to_string()
                            } else {
                                "Warning".to_string()
                            };
                        }
                    }
                }
            } else {
                // Even if failed, check permission
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") {
                    health_status = "Root required".to_string();
                }
            }
        } else {
            health_status = "Smartctl not found".to_string();
        }

        storage_devices.push(StorageDetailedInfo {
            device_name,
            model,
            capacity_bytes,
            interface_type,
            is_ssd,
            serial_number,
            firmware_version,
            health_status,
        });
    }

    storage_devices
}

pub fn get_network_detailed_info_headless(networks: &Networks) -> Vec<NetworkDetailedInfo> {
    let mut networks_info = Vec::new();
    for (interface_name, data) in networks {
        // ... (Logic from get_network_detailed_info)
        let mac_address = data.mac_address().to_string();

        let mut ip_v4 = "N/A".to_string();
        let mut ip_v6 = "N/A".to_string();
        for ip in data.ip_networks() {
            match ip.addr {
                std::net::IpAddr::V4(addr) => ip_v4 = addr.to_string(),
                std::net::IpAddr::V6(addr) => ip_v6 = addr.to_string(),
            }
        }

        let speed_path = format!("/sys/class/net/{}/speed", interface_name);
        let link_speed = std::fs::read_to_string(&speed_path)
            .map(|s| format!("{} Mbps", s.trim()))
            .unwrap_or("Unknown".to_string());

        networks_info.push(NetworkDetailedInfo {
            name: interface_name.clone(),
            mac_address,
            rx_bytes: data.total_received(),
            tx_bytes: data.total_transmitted(),
            rx_packets: data.total_packets_received(),
            tx_packets: data.total_packets_transmitted(),
            ip_v4,
            ip_v6,
            link_speed,
        });
    }
    networks_info.sort_by(|a, b| a.name.cmp(&b.name));
    networks_info
}
