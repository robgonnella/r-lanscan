use std::{collections::HashMap, env};

use serde::{Deserialize, Serialize};

use crate::ui::colors::Theme;

pub const DEFAULT_CONFIG_ID: &str = "default";
pub const DEFAULT_PORTS_STR: &str = "22,80,443,2000-9999";
pub const DEFAULT_PORTS: [&str; 4] = ["22", "80", "443", "2000-9999"];

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub ssh_port: u16,
    pub ssh_identity_file: String,
    pub ssh_user: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub cidr: String,
    pub theme: String,
    pub ports: Vec<String>,
    pub default_ssh_user: String,
    pub default_ssh_port: String,
    pub default_ssh_identity: String,
    pub device_configs: HashMap<String, DeviceConfig>,
}

pub fn get_default_ports() -> Vec<String> {
    DEFAULT_PORTS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<String>>()
}

impl Config {
    pub fn default() -> Self {
        let user = env::var("USER").unwrap();
        let home = env::var("HOME").unwrap();
        let identity = format!("{home}/.ssh/id_rsa");

        Self {
            id: DEFAULT_CONFIG_ID.to_string(),
            theme: Theme::Blue.to_string(),
            cidr: "unknown".to_string(),
            ports: get_default_ports(),
            default_ssh_identity: identity,
            default_ssh_port: String::from("22"),
            default_ssh_user: user,
            device_configs: HashMap::new(),
        }
    }
}

pub struct ConfigManager {
    path: String,
    configs: HashMap<String, Config>,
}

impl ConfigManager {
    pub fn new(path: &str) -> Self {
        let f: Result<std::fs::File, std::io::Error> = std::fs::File::open(&path);

        match f {
            Ok(file) => {
                let configs: HashMap<String, Config> = serde_yaml::from_reader(file).unwrap();
                Self {
                    path: String::from(path),
                    configs,
                }
            }
            Err(_) => {
                let default_conf = Config::default();
                let mut configs: HashMap<String, Config> = HashMap::new();
                configs.insert(default_conf.id.clone(), default_conf.clone());
                let mut man = Self {
                    path: String::from(path),
                    configs,
                };
                man.write();
                man
            }
        }
    }

    pub fn get_by_id(&self, id: &str) -> Option<Config> {
        let c = self.configs.get(id);
        match c {
            Some(conf) => Some(conf.clone()),
            None => None,
        }
    }

    pub fn get_by_cidr(&self, cidr: &str) -> Option<Config> {
        let mut config: Option<Config> = None;

        self.configs.iter().for_each(|(_, c)| {
            if c.cidr == *cidr {
                config = Some(c.clone());
                return;
            }
        });

        config
    }

    pub fn create(&mut self, config: &Config) {
        self.configs.insert(config.id.clone(), config.clone());
        self.write();
    }

    pub fn update_config(&mut self, new_config: Config) {
        self.configs.insert(new_config.id.clone(), new_config);
        self.write();
    }

    fn write(&mut self) {
        let serialized = serde_yaml::to_string(&self.configs).unwrap();
        std::fs::write(&self.path, serialized).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use nanoid::nanoid;
    use std::fs;

    use super::*;

    fn setup() -> (ConfigManager, Config, String) {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let mut manager = ConfigManager::new(tmp_path.as_str());
        let config = Config {
            id: "octopus".to_string(),
            cidr: "192.168.1.1/24".to_string(),
            default_ssh_identity: "id_rsa".to_string(),
            default_ssh_port: "2222".to_string(),
            default_ssh_user: "user".to_string(),
            device_configs: HashMap::new(),
            ports: vec![],
            theme: "Emerald".to_string(),
        };
        manager.create(&config);

        (manager, config, tmp_path)
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_new() {
        let (_, _, conf_path) = setup();
        assert!(true);
        tear_down(conf_path);
    }

    #[test]
    fn test_get_by_id() {
        let (manager, _, conf_path) = setup();
        let o = manager.get_by_id("default");
        assert!(o.is_some());
        let c = o.unwrap();
        assert_eq!(c.id, "default");
        tear_down(conf_path);

        let o = manager.get_by_id("nope");
        assert!(o.is_none());
    }

    #[test]
    fn get_by_cidr() {
        let (manager, config, conf_path) = setup();
        let o = manager.get_by_cidr(config.cidr.as_str());
        assert!(o.is_some());
        let c = o.unwrap();
        assert_eq!(c.id, config.id);
        tear_down(conf_path);
    }

    #[test]
    fn update_config() {
        let (mut manager, mut config, conf_path) = setup();
        config.cidr = "10.10.10.1/24".to_string();
        manager.update_config(config);
        let o = manager.get_by_id("octopus");
        assert!(o.is_some());
        let c = o.unwrap();
        assert_eq!(c.cidr, "10.10.10.1/24");
        tear_down(conf_path);
    }
}
