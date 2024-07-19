use std::sync;

use super::LazyLooper;

#[derive(Debug)]
pub struct PortTargets(Vec<String>);

pub fn new(list: Vec<String>) -> sync::Arc<PortTargets> {
    sync::Arc::new(PortTargets(list))
}

impl LazyLooper<u16> for PortTargets {
    fn lazy_loop<F: FnMut(u16)>(&self, mut cb: F) {
        for target in self.0.iter() {
            if target.contains("-") {
                let parts: Vec<&str> = target.split("-").collect();
                let begin = parts[0].parse::<u16>().unwrap();
                let end = parts[1].parse::<u16>().unwrap();
                for port in begin..end {
                    cb(port)
                }
            } else {
                let port = target.parse::<u16>().unwrap();
                cb(port)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_new_port_targets() {
        let list = vec![String::from("1"), String::from("2"), String::from("3")];
        let targets = new(list);
        assert!(!targets.0.is_empty());
    }

    #[test]
    fn lazy_loops_ports() {
        let list = vec![String::from("1"), String::from("2-4")];

        let expected = [1, 2, 3, 4];

        let targets = new(list);

        let mut idx = 0;

        let assert_ports = |port: u16| {
            assert_eq!(port, expected[idx]);
            idx += 1;
        };

        targets.lazy_loop(assert_ports);
    }
}
