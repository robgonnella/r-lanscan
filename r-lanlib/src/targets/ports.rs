use std::sync::Arc;

use crate::scanners::ScanError;

use super::LazyLooper;

#[derive(Debug)]
pub struct PortTargets(Vec<String>, usize);

fn loop_ports<F: FnMut(u16) -> Result<(), ScanError>>(
    list: &Vec<String>,
    mut cb: F,
) -> Result<(), ScanError> {
    for target in list.iter() {
        if target.contains("-") {
            let parts: Vec<&str> = target.split("-").collect();
            let begin = parts[0].parse::<u16>().or_else(|e| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: Some(target.to_string()),
                    msg: e.to_string(),
                })
            })?;
            let end = parts[1].parse::<u16>().or_else(|e| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: Some(target.to_string()),
                    msg: e.to_string(),
                })
            })?;
            for port in begin..end {
                if let Err(err) = cb(port) {
                    return Err(err);
                }
            }
        } else {
            let port = target.parse::<u16>().or_else(|e| {
                Err(ScanError {
                    ip: "".to_string(),
                    port: Some(target.to_string()),
                    msg: e.to_string(),
                })
            })?;
            if let Err(err) = cb(port) {
                return Err(err);
            }
        }
    }

    Ok(())
}

impl PortTargets {
    pub fn new(list: Vec<String>) -> Arc<Self> {
        let mut len = 0;
        loop_ports(&list, |_| {
            len += 1;
            Ok(())
        })
        .unwrap();
        Arc::new(Self(list, len))
    }
}

impl LazyLooper<u16> for PortTargets {
    fn len(&self) -> usize {
        self.1
    }

    fn lazy_loop<F: FnMut(u16) -> Result<(), ScanError>>(&self, cb: F) -> Result<(), ScanError> {
        loop_ports(&self.0, cb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_new_port_targets() {
        let list = vec![String::from("1"), String::from("2"), String::from("3")];
        let targets = PortTargets::new(list);
        assert!(!targets.0.is_empty());
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
}
