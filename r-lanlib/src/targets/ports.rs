//! Provides helpers for managing port target lists

use std::sync::Arc;

use crate::scanners::ScanError;

#[derive(Debug)]
/// Represents a list of Port targets
///
/// This wrapper is used to cut down on the memory needed to store entire
/// port ranges. Rather than storing all ports in a range of 1-65535, this
/// wrapper allows the storage of just the range in string form and then
/// dynamically loops the ports in that range when needed.
///
/// # Panics
///
/// Panics if an item in the list does not parse to u16
///
/// # Examples
///
/// ```rust
/// let ports = PortTargets::new(vec!["1-65535".to_string()])?;
/// ```
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
    /// Returns a new instance of PortTargets using the provided list
    pub fn new(list: Vec<String>) -> Arc<Self> {
        let mut len = 0;
        loop_ports(&list, |_| {
            len += 1;
            Ok(())
        })
        .unwrap();
        Arc::new(Self(list, len))
    }

    /// Returns the true length of the target list. If the underlying
    /// `Vec<String>` is just `["22-24"]`, then a call to "len" will
    /// return 3
    pub fn len(&self) -> usize {
        self.1
    }

    /// loops over all targets including those that are not explicitly in the
    /// list but fall within a range defined in the list
    pub fn lazy_loop<F: FnMut(u16) -> Result<(), ScanError>>(
        &self,
        cb: F,
    ) -> Result<(), ScanError> {
        loop_ports(&self.0, cb)
    }
}

#[cfg(test)]
#[path = "./ports_tests.rs"]
mod tests;
