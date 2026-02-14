use crate::monitor::{NetworkDetailedInfo, StorageDetailedInfo};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::{thread, time::Duration};
// Re-use logic from monitor or extract common logic?
// Ideally, `worker` should just use `monitor`'s functions but print result instead of storing in struct.
// But `Monitor` struct is tied to Slint `Weak<AppWindow>`.
// So we need a headless data gatherer.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivilegedData {
    pub storage: Vec<StorageDetailedInfo>,
    pub network: Vec<NetworkDetailedInfo>,
    // Add other fields if needed, e.g. DMI
}

pub fn run_worker() {
    // This runs as root
    let mut system = sysinfo::System::new_all();
    let mut networks = sysinfo::Networks::new_with_refreshed_list();

    loop {
        system.refresh_all();
        networks.refresh(true);

        // 1. Storage (Privileged: SMART)
        let storage_details = crate::monitor::get_storage_detailed_info_headless();

        // 2. Network (Privileged: Speed? Actually non-privileged usually fine, but consistent)
        let network_details = crate::monitor::get_network_detailed_info_headless(&networks);

        // 3. Serialize
        let data = PrivilegedData {
            storage: storage_details,
            network: network_details,
        };

        if let Ok(json) = serde_json::to_string(&data) {
            println!("{}", json);
            io::stdout().flush().unwrap();
        }

        thread::sleep(Duration::from_secs(2));
    }
}
