use color_eyre::eyre::{Result, eyre};
use r_lanlib::{
    network::{self, NetworkInterface},
    packet::{Reader as WireReader, Sender as WireSender},
    scanners::{
        Device, IDLE_TIMEOUT, PortSet, ScanMessage, Scanner,
        arp_scanner::{ARPScanner, ARPScannerArgs},
        syn_scanner::{SYNScanner, SYNScannerArgs},
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
    ui::store::{Dispatcher, Store, action::Action, derived::get_detected_arp_devices},
};

pub fn process_arp(
    args: ARPScannerArgs,
    rx: Receiver<ScanMessage>,
    dispatcher: Arc<dyn Dispatcher>,
) -> Result<Receiver<ScanMessage>> {
    let scanner = ARPScanner::new(args);

    dispatcher.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing ARP Scan…",
    ))));

    let handle = scanner.scan();

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

pub fn process_syn(
    args: SYNScannerArgs,
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

    let scanner = SYNScanner::new(args);

    log::debug!("starting syn scan");
    store.dispatch(Action::UpdateMessage(Some(String::from(
        "Performing SYN Scan…",
    ))));

    let handle = scanner.scan();

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
                        log::warn!("received syn result for unknown device: {:?}", device);
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
        let res = exit.try_recv();

        if res.is_ok() {
            return Ok(());
        }

        let source_port = network::get_available_port()?;

        let (tx, rx) = mpsc::channel::<ScanMessage>();

        let rx = process_arp(
            ARPScannerArgs {
                interface: &interface,
                packet_reader: Arc::clone(&packet_reader),
                packet_sender: Arc::clone(&packet_sender),
                targets: IPTargets::new(vec![interface.cidr.clone()])
                    .map_err(|e| eyre!("Invalid IP targets: {}", e))?,
                include_host_names: true,
                include_vendor: true,
                idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
                source_port,
                notifier: tx.clone(),
            },
            rx,
            Arc::clone(&store) as Arc<dyn Dispatcher>,
        )?;

        let state = store.get_state()?;
        let arp_devices = get_detected_arp_devices(&state);

        let results = process_syn(
            SYNScannerArgs {
                interface: &interface,
                packet_reader: Arc::clone(&packet_reader),
                packet_sender: Arc::clone(&packet_sender),
                targets: arp_devices,
                ports: PortTargets::new(config.ports.clone())
                    .map_err(|e| eyre!("Invalid port targets: {}", e))?,
                source_port,
                idle_timeout: time::Duration::from_millis(IDLE_TIMEOUT.into()),
                notifier: tx.clone(),
            },
            rx,
            Arc::clone(&store),
        )?;

        store.dispatch(Action::UpdateAllDevices(results));

        log::debug!("network scan completed");

        thread::sleep(time::Duration::from_secs(15));
    }
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;
