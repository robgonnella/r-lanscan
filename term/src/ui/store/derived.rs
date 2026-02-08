//! Derived state selectors for computed values.

use r_lanlib::scanners::Device;

use super::state::State;

/// Returns devices detected in the last ARP scan (miss count = 0).
pub fn get_detected_arp_devices(state: &State) -> Vec<Device> {
    state
        .arp_history
        .iter()
        .filter(|d| d.1.1 == 0)
        .map(|d| d.1.0.clone())
        .collect::<Vec<Device>>()
}

#[cfg(test)]
#[path = "./derived_tests.rs"]
mod tests;
