//! Configuration management for SSH, ports, and per-device settings.

use std::collections::HashMap;

use color_eyre::eyre::Result;
use derive_builder::Builder;
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

/// Returns the default ports to scan as a vector of strings.
pub fn get_default_ports() -> Vec<String> {
    DEFAULT_PORTS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<String>>()
}

impl Config {
    /// Creates a new config with defaults for the given user, identity, and
    /// network CIDR.
    pub fn new(user: String, identity: String, cidr: String) -> Self {
        Self {
            id: DEFAULT_CONFIG_ID.to_string(),
            theme: Theme::Blue.to_string(),
            cidr,
            ports: get_default_ports(),
            default_ssh_identity: identity,
            default_ssh_port: 22,
            default_ssh_user: user,
            device_configs: HashMap::new(),
        }
    }
}

/// Persists and retrieves configurations from YAML file.
#[derive(Builder)]
#[builder(setter(into), build_fn(private, name = "_build"))]
pub struct ConfigManager {
    /// The default user to use when config is not found for target network
    default_user: String,
    /// The default ssh identity file to use when config is not found for
    /// target network
    default_identity: String,
    /// The current network cidr to use when no saved config is found
    default_cidr: String,
    /// The path the config file
    path: String,
    #[builder(setter(skip))]
    configs: HashMap<String, Config>,
}

impl ConfigManagerBuilder {
    pub fn build(&self) -> Result<ConfigManager> {
        let mut manager = self._build()?;

        let f: Result<std::fs::File, std::io::Error> = std::fs::File::open(&manager.path);

        match f {
            Ok(file) => {
                manager.configs = match serde_yaml::from_reader(file) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to parse config file, using defaults: {}", e);
                        let default_conf = Config::new(
                            manager.default_user.clone(),
                            manager.default_identity.clone(),
                            manager.default_cidr.clone(),
                        );
                        let mut configs: HashMap<String, Config> = HashMap::new();
                        configs.insert(default_conf.id.clone(), default_conf);
                        configs
                    }
                };
                Ok(manager)
            }
            Err(_) => {
                let default_conf = Config::new(
                    manager.default_user.clone(),
                    manager.default_identity.clone(),
                    manager.default_cidr.clone(),
                );
                let mut configs: HashMap<String, Config> = HashMap::new();
                configs.insert(default_conf.id.clone(), default_conf.clone());
                manager.configs = configs;
                manager.write()?;
                Ok(manager)
            }
        }
    }
}

impl ConfigManager {
    /// Returns a new instance of ConfigManagerBuilder.
    pub fn builder() -> ConfigManagerBuilder {
        ConfigManagerBuilder::default()
    }

    /// Retrieves a config by its unique ID.
    pub fn get_by_id(&self, id: &str) -> Option<Config> {
        let c = self.configs.get(id);
        c.cloned()
    }

    /// Retrieves a config by network CIDR.
    pub fn get_by_cidr(&self, cidr: &str) -> Option<Config> {
        let mut config: Option<Config> = None;

        self.configs.iter().for_each(|(_, c)| {
            if c.cidr == *cidr {
                config = Some(c.clone());
            }
        });

        config
    }

    /// Creates a new config and persists it to disk.
    pub fn create(&mut self, config: &Config) -> Result<()> {
        self.configs.insert(config.id.clone(), config.clone());
        self.write()
    }

    /// Updates an existing config and persists it to disk.
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
