use nanoid::nanoid;
use std::fs;

use super::*;

fn setup() -> (ConfigManager, Config, String) {
    fs::create_dir_all("generated").unwrap();
    let tmp_path = format!("generated/{}.yml", nanoid!());
    let user = "user".to_string();
    let identity = "/home/user/.ssh/id_rsa".to_string();
    let mut manager = ConfigManager::new(user, identity, tmp_path.as_str());
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
