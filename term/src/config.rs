//! Configuration management for SSH, ports, and per-device settings.

use std::collections::HashMap;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

use crate::ui::colors::Theme;

pub const DEFAULT_CONFIG_ID: &str = "default";
pub const DEFAULT_PORTS_STR: &str = "22,80,443,2000-9999,27017";
pub const DEFAULT_PORTS: [&str; 5] = ["22", "80", "443", "2000-9999", "27017"];

/// SSH configuration for a specific device.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub ssh_port: u16,
    pub ssh_identity_file: String,
    pub ssh_user: String,
}

/// Application configuration for a network (CIDR).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub cidr: String,
    pub theme: String,
    pub ports: Vec<String>,
    pub default_ssh_user: String,
    pub default_ssh_port: u16,
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
    pub fn new(user: String, identity: String) -> Self {
        Self {
            id: DEFAULT_CONFIG_ID.to_string(),
            theme: Theme::Blue.to_string(),
            cidr: "unknown".to_string(),
            ports: get_default_ports(),
            default_ssh_identity: identity,
            default_ssh_port: 22,
            default_ssh_user: user,
            device_configs: HashMap::new(),
        }
    }
}

/// Persists and retrieves configurations from YAML file.
pub struct ConfigManager {
    path: String,
    configs: HashMap<String, Config>,
}

impl ConfigManager {
    pub fn new(user: String, identity: String, path: &str) -> Result<Self> {
        let f: Result<std::fs::File, std::io::Error> = std::fs::File::open(path);

        match f {
            Ok(file) => {
                let configs: HashMap<String, Config> = match serde_yaml::from_reader(file) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to parse config file, using defaults: {}", e);
                        let default_conf = Config::new(user, identity);
                        let mut configs: HashMap<String, Config> = HashMap::new();
                        configs.insert(default_conf.id.clone(), default_conf);
                        configs
                    }
                };
                Ok(Self {
                    path: String::from(path),
                    configs,
                })
            }
            Err(_) => {
                let default_conf = Config::new(user, identity);
                let mut configs: HashMap<String, Config> = HashMap::new();
                configs.insert(default_conf.id.clone(), default_conf.clone());
                let mut man = Self {
                    path: String::from(path),
                    configs,
                };
                man.write()?;
                Ok(man)
            }
        }
    }

    pub fn get_by_id(&self, id: &str) -> Option<Config> {
        let c = self.configs.get(id);
        c.cloned()
    }

    pub fn get_by_cidr(&self, cidr: &str) -> Option<Config> {
        let mut config: Option<Config> = None;

        self.configs.iter().for_each(|(_, c)| {
            if c.cidr == *cidr {
                config = Some(c.clone());
            }
        });

        config
    }

    pub fn create(&mut self, config: &Config) -> Result<()> {
        self.configs.insert(config.id.clone(), config.clone());
        self.write()
    }

    pub fn update_config(&mut self, new_config: Config) -> Result<()> {
        self.configs.insert(new_config.id.clone(), new_config);
        self.write()
    }

    fn write(&mut self) -> Result<()> {
        let serialized = serde_yaml::to_string(&self.configs)?;
        std::fs::write(&self.path, serialized)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "./config_tests.rs"]
mod tests;
