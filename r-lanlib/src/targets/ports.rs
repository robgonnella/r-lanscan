use std::sync::Arc;

use crate::scanners::ScanError;

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
                    ip: None,
                    port: Some(target.to_string()),
                    error: Box::from(e),
                })
            })?;
            let end = parts[1].parse::<u16>().or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: Some(target.to_string()),
                    error: Box::from(e),
                })
            })?;
            for port in begin..=end {
                cb(port)?;
            }
        } else {
            let port = target.parse::<u16>().or_else(|e| {
                Err(ScanError {
                    ip: None,
                    port: Some(target.to_string()),
                    error: Box::from(e),
                })
            })?;
            cb(port)?;
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

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn lazy_loop<F: FnMut(u16) -> Result<(), ScanError>>(
        &self,
        cb: F,
    ) -> Result<(), ScanError> {
        loop_ports(&self.0, cb)
    }
}

#[cfg(test)]
#[path = "./tests/ports_tests.rs"]
mod tests;
