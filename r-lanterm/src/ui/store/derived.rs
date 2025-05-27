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
            ssh_port: state
                .config
                .default_ssh_port
                .clone()
                .parse::<u16>()
                .unwrap(),
            ssh_user: state.config.default_ssh_user.clone(),
        })
}

// returns just the devices that were detected in last arp scan
// i.e. miss count = 0
pub fn get_detected_devices(state: &State) -> Vec<Device> {
    state
        .arp_history
        .iter()
        .filter(|d| d.1 .1 == 0)
        .map(|d| d.1 .0.clone())
        .collect::<Vec<Device>>()
}

#[cfg(test)]
mod tests {
    use nanoid::nanoid;
    use pnet::util::MacAddr;
    use r_lanlib::scanners::DeviceWithPorts;
    use std::{
        collections::HashSet,
        fs,
        sync::{Arc, Mutex},
    };

    use crate::{
        config::{Config, ConfigManager},
        ui::store::{action::Action, store::Store},
    };

    use super::*;

    fn tear_down(conf_path: &str) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_get_device_config_from_state() {
        let device_1 = DeviceWithPorts {
            ip: "10.10.10.1".to_string(),
            mac: MacAddr::default().to_string(),
            hostname: "fancy_hostname".to_string(),
            vendor: "mac".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        let device_2 = DeviceWithPorts {
            ip: "10.10.10.2".to_string(),
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            hostname: "super_fancy_hostname".to_string(),
            vendor: "linux".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));

        let config = Config::default();
        let store = Store::new(conf_manager);
        let devices = vec![device_1.clone(), device_2.clone()];

        store.dispatch(Action::CreateAndSetConfig(config.clone()));
        store.dispatch(Action::UpdateAllDevices(devices));
        store.dispatch(Action::UpdateDeviceConfig(DeviceConfig {
            id: device_1.mac.clone(),
            ssh_port: 2222,
            ssh_identity_file: "dev_1_id_rsa".to_string(),
            ssh_user: "dev1_user".to_string(),
        }));
        store.dispatch(Action::UpdateSelectedDevice(device_1.mac.clone()));

        let state = store.get_state();

        let dev1_config = get_selected_device_config_from_state(&state);

        assert_eq!(dev1_config.id, device_1.mac);
        assert_eq!(dev1_config.ssh_port, 2222);
        assert_eq!(dev1_config.ssh_identity_file, "dev_1_id_rsa");
        assert_eq!(dev1_config.ssh_user, "dev1_user");

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn test_get_device_config_from_state_default() {
        let device_1 = DeviceWithPorts {
            ip: "10.10.10.1".to_string(),
            mac: MacAddr::default().to_string(),
            hostname: "fancy_hostname".to_string(),
            vendor: "mac".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        let device_2 = DeviceWithPorts {
            ip: "10.10.10.2".to_string(),
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            hostname: "super_fancy_hostname".to_string(),
            vendor: "linux".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));

        let config = Config::default();
        let store = Store::new(conf_manager);
        let devices = vec![device_1.clone(), device_2.clone()];

        store.dispatch(Action::CreateAndSetConfig(config.clone()));
        store.dispatch(Action::UpdateAllDevices(devices));
        store.dispatch(Action::UpdateDeviceConfig(DeviceConfig {
            id: device_1.mac.clone(),
            ssh_port: 2222,
            ssh_identity_file: "dev_1_id_rsa".to_string(),
            ssh_user: "dev1_user".to_string(),
        }));

        let state = store.get_state();

        let dev1_config = get_selected_device_config_from_state(&state);

        assert_eq!(dev1_config.id, MacAddr::default().to_string());
        assert_eq!(dev1_config.ssh_port.to_string(), config.default_ssh_port);
        assert_eq!(dev1_config.ssh_identity_file, config.default_ssh_identity);
        assert_eq!(dev1_config.ssh_user, config.default_ssh_user);

        tear_down(tmp_path.as_str());
    }

    #[test]
    fn test_get_detected_devices() {
        let device_1 = DeviceWithPorts {
            ip: "10.10.10.1".to_string(),
            mac: MacAddr::default().to_string(),
            hostname: "fancy_hostname".to_string(),
            vendor: "mac".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        let device_2 = DeviceWithPorts {
            ip: "10.10.10.2".to_string(),
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            hostname: "super_fancy_hostname".to_string(),
            vendor: "linux".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        let device_3 = DeviceWithPorts {
            ip: "10.10.10.3".to_string(),
            mac: "aa:aa:aa:aa:aa:aa".to_string(),
            hostname: "just_ok_hostname".to_string(),
            vendor: "linux".to_string(),
            is_current_host: false,
            open_ports: HashSet::new(),
        };

        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));

        let config = Config::default();
        let store = Store::new(conf_manager);

        store.dispatch(Action::CreateAndSetConfig(config.clone()));

        // this updates arp history
        store.dispatch(Action::AddDevice(device_1.clone()));
        store.dispatch(Action::AddDevice(device_2.clone()));
        store.dispatch(Action::AddDevice(device_3.clone()));

        store.dispatch(Action::UpdateAllDevices(vec![
            device_1.clone(),
            device_2.clone(),
            device_3.clone(),
        ]));

        // missed dev2 & dev3
        store.dispatch(Action::UpdateAllDevices(vec![device_1.clone()]));

        // missed dev3
        store.dispatch(Action::UpdateAllDevices(vec![
            device_1.clone(),
            device_2.clone(),
        ]));

        // missed dev3 again
        store.dispatch(Action::UpdateAllDevices(vec![
            device_1.clone(),
            device_2.clone(),
        ]));

        // missed dev3 again
        store.dispatch(Action::UpdateAllDevices(vec![
            device_1.clone(),
            device_2.clone(),
        ]));

        // get just devices with miss count = 0 -> device_1
        let arp_devices = get_detected_devices(&store.get_state());
        assert_eq!(arp_devices.len(), 1);
        assert_eq!(arp_devices[0], device_1.into());
        tear_down(tmp_path.as_str());
    }
}
