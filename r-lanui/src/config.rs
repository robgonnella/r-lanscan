use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ui::store::types::Theme;

pub const DEFAULT_CONFIG_ID: &str = "default";

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum SshID {
    Ip(String),
    Mac(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SshOverride {
    pub id: SshID,
    pub port: u16,
    pub identity_file: String,
    pub user: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub cidr: String,
    pub theme: String,
    pub ssh_overrides: HashMap<SshID, SshOverride>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            id: DEFAULT_CONFIG_ID.to_string(),
            theme: Theme::Blue.to_string(),
            cidr: "unknown".to_string(),
            ssh_overrides: HashMap::new(),
        }
    }
}

pub struct ConfigManager {
    path: String,
    configs: HashMap<String, Config>,
}

impl ConfigManager {
    pub fn new(path: String) -> Self {
        let f: Result<std::fs::File, std::io::Error> = std::fs::File::open(&path);

        match f {
            Ok(file) => {
                let configs: HashMap<String, Config> = serde_yaml::from_reader(file).unwrap();
                Self { path, configs }
            }
            Err(_) => {
                let default_conf = Config::new();
                let mut configs: HashMap<String, Config> = HashMap::new();
                configs.insert(default_conf.id.clone(), default_conf.clone());
                let mut man = Self { path, configs };
                man.write();
                man
            }
        }
    }

    pub fn get_by_id(&self, id: &String) -> Option<Config> {
        let c = self.configs.get(id);
        match c {
            Some(conf) => Some(conf.clone()),
            None => None,
        }
    }

    pub fn get_by_cidr(&self, cidr: &String) -> Option<Config> {
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

    pub fn update_theme(&mut self, id: &String, theme: &Theme) {
        if let Some(conf) = self.configs.get_mut(id) {
            conf.theme = theme.clone().to_string();
            self.write();
        }
    }

    fn write(&mut self) {
        let serialized = serde_yaml::to_string(&self.configs).unwrap();
        std::fs::write(&self.path, serialized).unwrap();
    }
}
