use pnet::{
    datalink::NetworkInterface as PNetNetworkInterface, ipnetwork::IpNetwork, util::MacAddr,
};
use std::{
    error::Error,
    io,
    net::{Ipv4Addr, TcpListener},
    str::FromStr,
};

pub struct NetworkInterface {
    pub name: String,
    pub description: String,
    pub cidr: String,
    pub ipv4: Ipv4Addr,
    pub ips: Vec<IpNetwork>,
    pub mac: MacAddr,
    pub flags: u32,
    pub index: u32,
}

impl TryFrom<PNetNetworkInterface> for NetworkInterface {
    type Error = Box<dyn Error>;

    fn try_from(value: PNetNetworkInterface) -> Result<Self, Self::Error> {
        let mac = value.mac.ok_or("failed to get mac address for interface")?;
        let (ip, cidr) =
            get_interface_ipv4_and_cidr(&value).ok_or("failed to get ip and cidr for interface")?;
        let ipv4 = Ipv4Addr::from_str(ip.as_str())?;

        Ok(Self {
            name: value.name,
            description: value.description,
            flags: value.flags,
            index: value.index,
            mac,
            ips: value.ips,
            cidr,
            ipv4,
        })
    }
}

impl From<&NetworkInterface> for PNetNetworkInterface {
    fn from(value: &NetworkInterface) -> Self {
        Self {
            name: value.name.clone(),
            flags: value.flags.clone(),
            description: value.description.clone(),
            index: value.index.clone(),
            ips: value.ips.clone(),
            mac: Some(value.mac.clone()),
        }
    }
}

pub fn get_interface(name: &str) -> Option<NetworkInterface> {
    let iface = pnet::datalink::interfaces()
        .into_iter()
        .find(|i| i.name == name)?;
    NetworkInterface::try_from(iface).ok()
}

pub fn get_default_interface() -> Option<NetworkInterface> {
    let iface = pnet::datalink::interfaces()
        .into_iter()
        .find(|e| e.is_up() && !e.is_loopback() && e.ips.iter().find(|i| i.is_ipv4()).is_some())?;
    NetworkInterface::try_from(iface).ok()
}

pub fn get_available_port() -> Result<u16, io::Error> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let addr = listener.local_addr()?;
    Ok(addr.port())
}

fn get_interface_ipv4_and_cidr(interface: &PNetNetworkInterface) -> Option<(String, String)> {
    let ipnet = interface.ips.iter().find(|i| i.is_ipv4())?;
    let ip = ipnet.ip().to_string();
    let base = ipnet.network().to_string();
    let prefix = ipnet.prefix().to_string();
    let cidr = String::from(format!("{base}/{prefix}"));
    Some((ip, cidr))
}
