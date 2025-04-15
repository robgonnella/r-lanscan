use r_lanlib::scanners::DeviceWithPorts;

use crate::config::DeviceConfig;

use super::state::State;

pub fn get_device_config_from_state(device: &DeviceWithPorts, state: &State) -> DeviceConfig {
    state
        .selected_device_config
        .clone()
        .unwrap_or(DeviceConfig {
            id: device.mac.clone(),
            ssh_identity_file: state.config.default_ssh_identity.clone(),
            ssh_port: state
                .config
                .default_ssh_port
                .clone()
                .parse::<u16>()
                .unwrap(),
            ssh_user: state.config.default_ssh_user.clone(),
        })
}
