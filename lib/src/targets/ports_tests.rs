use super::*;

#[test]
fn returns_new_port_targets() {
    let list = vec![String::from("1"), String::from("2"), String::from("3")];
    let targets = PortTargets::new(list);
    assert!(!targets.0.is_empty());
}

#[test]
fn returns_port_target_len() {
    let list = vec![String::from("1"), String::from("2"), String::from("3-5")];
    let targets = PortTargets::new(list);
    assert_eq!(targets.len(), 5);
}

#[test]
fn lazy_loops_ports() {
    let list = vec![String::from("1"), String::from("2-4")];

    let expected = [1, 2, 3, 4];

    let targets = PortTargets::new(list);

    let mut idx = 0;

    let assert_ports = |port: u16| {
        assert_eq!(port, expected[idx]);
        idx += 1;
        Ok(())
    };

    targets.lazy_loop(assert_ports).unwrap();
}

#[test]
#[should_panic]
fn returns_error_for_malformed_port() {
    let list = vec![String::from("nope")];
    let _targets = PortTargets::new(list);
}

#[test]
#[should_panic]
fn returns_error_for_malformed_range_start() {
    let list = vec![String::from("nope-3")];
    let _targets = PortTargets::new(list);
}

#[test]
#[should_panic]
fn returns_error_for_malformed_range_end() {
    let list = vec![String::from("4-nope")];
    let _targets = PortTargets::new(list);
}
