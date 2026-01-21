use pnet::util::MacAddr;
use r_lanlib::scanners::Device;

use crate::config::DeviceConfig;

use super::state::State;

pub fn get_selected_device_config_from_state(state: &State) -> DeviceConfig {
    state
        .selected_device_config
        .clone()
        .unwrap_or(DeviceConfig {
            id: MacAddr::default().to_string(),
            ssh_identity_file: state.config.default_ssh_identity.clone(),
            ssh_port: state.config.default_ssh_port,
            ssh_user: state.config.default_ssh_user.clone(),
        })
}

// returns just the devices that were detected in last arp scan
// i.e. miss count = 0
pub fn get_detected_arp_devices(state: &State) -> Vec<Device> {
    state
        .arp_history
        .iter()
        .filter(|d| d.1.1 == 0)
        .map(|d| d.1.0.clone())
        .collect::<Vec<Device>>()
}

#[cfg(test)]
#[path = "./derived_tests.rs"]
mod tests;
