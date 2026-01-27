//! Network monitoring and scanning orchestration.
//!
//! Runs continuous ARP and SYN scans to discover devices and open ports on
//! the local network.

use color_eyre::eyre::{Result, eyre};
use r_lanlib::{
    network::{self, NetworkInterface},
    packet::{Reader as WireReader, Sender as WireSender},
    scanners::{
        Device, IDLE_TIMEOUT, PortSet, ScanMessage, Scanner,
        arp_scanner::ARPScanner, syn_scanner::SYNScanner,
    },
    targets::{ips::IPTargets, ports::PortTargets},
};
use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread, time,
};

use crate::{
    config::Config,
    error,
    ui::store::{
        Dispatcher, Store, action::Action, derived::get_detected_arp_devices,
    },
};

/// Main network monitoring loop. Continuously runs ARP and SYN scans,
/// updating the store with discovered devices.
pub fn monitor_network(
    exit: Receiver<()>,
    packet_reader: Arc<Mutex<dyn WireReader>>,
    packet_sender: Arc<Mutex<dyn WireSender>>,
    config: Config,
    interface: Arc<NetworkInterface>,
    store: Arc<Store>,
) -> Result<()> {
    log::info!("starting network monitor");

    loop {
        if exit.try_recv().is_ok() {
            return Ok(());
        }

        let source_port = network::get_available_port()?;

        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let ip_targets = IPTargets::new(vec![interface.cidr.clone()])
            .map_err(|e| eyre!("Invalid IP targets: {}", e))?;

        let arp_scanner = ARPScanner::builder()
            .interface(interface.as_ref())
            .packet_reader(Arc::clone(&packet_reader))
            .packet_sender(Arc::clone(&packet_sender))
            .targets(ip_targets)
            .include_host_names(true)
            .include_vendor(true)
            .idle_timeout(time::Duration::from_millis(IDLE_TIMEOUT.into()))
            .source_port(source_port)
            .notifier(tx.clone())
            .build()?;

        let rx = process_arp(
            arp_scanner,
            rx,
            Arc::clone(&store) as Arc<dyn Dispatcher>,
        )?;

        let state = store.get_state()?;
        let arp_devices = get_detected_arp_devices(&state);

        let port_targets = PortTargets::new(config.ports.clone())
            .map_err(|e| eyre!("Invalid port targets: {}", e))?;

        let syn_scanner = SYNScanner::builder()
            .interface(interface.as_ref())
            .packet_reader(Arc::clone(&packet_reader))
            .packet_sender(Arc::clone(&packet_sender))
            .targets(arp_devices)
            .ports(port_targets)
            .source_port(source_port)
            .idle_timeout(time::Duration::from_millis(IDLE_TIMEOUT.into()))
            .notifier(tx.clone())
            .build()?;

        let results = process_syn(syn_scanner, rx, Arc::clone(&store))?;

        store.dispatch(Action::UpdateAllDevices(results));

        log::debug!("network scan completed");

        thread::sleep(time::Duration::from_secs(15));
    }
}

/// Runs an ARP scan and dispatches discovered devices to the store.
fn process_arp(
    scanner: ARPScanner,
    rx: Receiver<ScanMessage>,
    dispatcher: Arc<dyn Dispatcher>,
) -> Result<Receiver<ScanMessage>> {
    dispatcher.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing ARP Scan…",
    ))));

    let handle = scanner.scan()?;

    loop {
        let msg = rx.recv()?;

        match msg {
            ScanMessage::Done => {
                log::debug!("scanning complete");
                break;
            }
            ScanMessage::ARPScanDevice(d) => {
                log::debug!("received scanning message: {:?}", d);
                dispatcher.dispatch(Action::AddDevice(d));
            }
            _ => {}
        }
    }

    log::debug!("waiting for arp handle to finish");

    handle.join().map_err(error::report_from_thread_panic)??;

    dispatcher.dispatch(Action::UpdateMessage(None));

    log::debug!("finished arp scan");
    Ok(rx)
}

/// Runs a SYN scan on discovered devices and returns devices with open ports.
fn process_syn(
    scanner: SYNScanner,
    rx: Receiver<ScanMessage>,
    store: Arc<Store>,
) -> Result<HashMap<Ipv4Addr, Device>> {
    let state = store.get_state()?;
    let arp_devices = get_detected_arp_devices(&state);
    let mut syn_results: HashMap<Ipv4Addr, Device> = HashMap::new();

    for d in arp_devices.iter() {
        syn_results.insert(
            d.ip,
            Device {
                hostname: d.hostname.to_owned(),
                ip: d.ip.to_owned(),
                mac: d.mac.to_owned(),
                vendor: d.vendor.to_owned(),
                is_current_host: d.is_current_host,
                open_ports: PortSet::new(),
            },
        );
    }

    log::debug!("starting syn scan");
    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing SYN Scan…",
    ))));

    let handle = scanner.scan()?;

    loop {
        let msg = rx.recv()?;

        match msg {
            ScanMessage::Done => {
                log::debug!("scanning complete");
                break;
            }
            ScanMessage::SYNScanDevice(device) => {
                log::debug!("received syn scanning device: {:?}", device);
                let result = syn_results.get_mut(&device.ip);
                match result {
                    Some(d) => {
                        d.open_ports.0.extend(device.open_ports.0);
                        store.dispatch(Action::AddDevice(d.clone()));
                    }
                    None => {
                        log::warn!(
                            "received syn result for unknown device: {:?}",
                            device
                        );
                    }
                }
            }
            _ => {}
        }
    }

    handle.join().map_err(error::report_from_thread_panic)??;

    store.dispatch(Action::UpdateMessage(None));

    Ok(syn_results)
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;
