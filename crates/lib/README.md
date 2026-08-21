# r-lanlib

A Rust library crate for performing network scanning operations on any local
area network (LAN). This is the Rust version of the
[go-lanscan package](https://github.com/robgonnella/go-lanscan).

## Features

- **ARP Scanning**: Discover devices on your network using Address Resolution Protocol
- **SYN Scanning**: Detect open ports on discovered devices using TCP SYN packets
- **Full Scanning**: Combined ARP and SYN scanning in a single operation
- **Vendor Detection**: Identify device manufacturers using MAC address lookup
- **Hostname Resolution**: Resolve hostnames for discovered devices
- **Async Communication**: Channel-based communication for real-time scan results
- **Flexible Targeting**: Support for CIDR blocks, IP ranges, and port ranges

## Requirements

- **Root privileges required**: This library performs raw packet operations that
  require elevated permissions
- **Rust 1.89.0+** with Rust 2024 edition support

## Installation

```bash
cargo add r-lanlib
```

## Quick Start

See the working examples in [`examples/`](./examples/):

- [`arp-scanner.rs`](./examples/arp-scanner.rs) - ARP device discovery
- [`syn-scanner.rs`](./examples/syn-scanner.rs) - Port scanning on known devices
- [`full-scanner.rs`](./examples/full-scanner.rs) - Combined ARP + SYN scanning

Run them from the workspace root:

```bash
sudo -E cargo run --example arp-scanner -p r-lanlib
sudo -E cargo run --example syn-scanner -p r-lanlib
sudo -E cargo run --example full-scanner -p r-lanlib
```

## API Reference

### Core Modules

#### `network`

Provides helpers for selecting network interfaces:

- `get_default_interface()` - Get the default network interface, returns `Result<NetworkInterface>`
- `get_interface(name)` - Get a specific interface by name, returns `Result<NetworkInterface>`
- `get_available_port()` - Find an available port for scanning, returns `Result<u16>`

#### `routing`

Provides OS-level routing table inspection:

- `get_default_gateway()` - Detect the default gateway IP address by parsing
  the system routing table (`netstat -rn` on macOS, `ip route show` on Linux).
  Returns `Option<Ipv4Addr>` — `None` if the gateway cannot be determined or
  the platform is unsupported.

#### `oui`

OUI (Organizationally Unique Identifier) lookup for resolving MAC address
prefixes to vendor/organization names:

- `oui::default(project_name, max_age)` - Initialize the built-in IEEE OUI
  database. Downloads and caches five IEEE CSV data files locally under the
  OS-appropriate data directory for `project_name`. Re-downloads automatically
  when the cached files are older than `max_age`. Returns
  `Result<Arc<dyn Oui>>`.
- `oui::traits::Oui` - Trait for custom OUI implementations. Implement this to
  supply your own vendor database to the scanners.
- `oui::db::OuiDb` - The default implementation backed by locally cached IEEE
  CSV files. Supports MA-L (24-bit), MA-M (28-bit), and MA-S/IAB (36-bit)
  prefixes, resolving the most-specific match first.
- `oui::types::OuiData` - Holds the `organization` string for a matched prefix.

#### `wire`

Low-level packet I/O:

- `wire::default(interface)` - Create a `Wire` for reading and sending packets
- Various packet builders for ARP, SYN, RST packets (in the `packet` module)

#### `scanners`

Main scanning implementations:

- `ARPScanner` - Discover devices using ARP
- `SYNScanner` - Scan ports on known devices
- `FullScanner` - Combined ARP + SYN scanning

#### `targets`

Target specification utilities:

- `ips::IPTargets` - Define IP ranges and CIDR blocks
- `ports::PortTargets` - Define port ranges and individual ports

### Data Structures

#### `Device`

Represents a discovered network device:

```rust
pub struct Device {
    pub hostname: String,
    pub ip: Ipv4Addr,
    pub mac: MacAddr,
    pub vendor: String,
    pub is_current_host: bool,
    pub is_gateway: bool,
    pub open_ports: PortSet,
    pub latency_ms: Option<u128>,
    pub response_ttl: Option<u8>,
}
```

#### `Port`

Represents a network port:

```rust
pub struct Port {
    pub id: u16,
    pub service: String,
}
```

#### `PortSet`

Wrapper around `HashSet<Port>` with convenience methods:

```rust
pub struct PortSet(pub HashSet<Port>);

impl PortSet {
    pub fn new() -> Self;
    pub fn to_sorted_vec(&self) -> Vec<Port>;
}
```

#### `ScanMessage`

Messages sent over the notification channel:

```rust
pub enum ScanMessage {
    Done,                    // Scanning complete
    Info(Scanning),          // Status update
    ARPScanDevice(Device),   // ARP discovery result
    SYNScanDevice(Device),   // SYN scan result (Device with open_ports populated)
}
```

### Target Specification

#### IP Targets

```rust
// CIDR blocks
IPTargets::new(vec!["192.168.1.0/24".to_string()]);

// IP ranges
IPTargets::new(vec!["192.168.1.1-192.168.1.100".to_string()]);

// Individual IPs
IPTargets::new(vec!["192.168.1.1".to_string(), "10.0.0.1".to_string()]);
```

#### Port Targets

```rust
// Port ranges
PortTargets::new(vec!["1-1000".to_string()]);

// Individual ports
PortTargets::new(vec!["22".to_string(), "80".to_string(), "443".to_string()]);

// Mixed specification
PortTargets::new(vec![
    "22".to_string(),
    "80".to_string(),
    "8000-9000".to_string()
]);
```

## Configuration Options

### Scanner Timeouts

- `idle_timeout` - How long to wait for responses before concluding scan
- Default: 10 seconds (10,000ms)
- Recommended: 5-30 seconds depending on network size and latency

### Packet Send Throttle

- `throttle` - Delay inserted between sending each packet
- Default: 200 microseconds (`Duration::from_micros(200)`)
- Higher values reduce packet loss on congested or high-latency networks at
  the cost of increased total scan time

### Scanner Features

- `include_vendor` - Enable MAC address vendor lookup (requires `oui` to be set)
- `oui` - Supply an `Arc<dyn Oui>` database for vendor lookups. Use
  `oui::default(project_name, max_age)` for the built-in IEEE database, or
  provide a custom implementation. When `None`, vendor lookup is skipped even
  if `include_vendor` is `true`.
- `include_host_names` - Resolve hostnames via reverse DNS lookup
- `source_port` - Source port for scan packets (auto-selected if not specified)
- `throttle` - Delay between sending packets (default: 200µs); increase for more
  accurate scans on lossy or congested networks

### Performance Tuning

- **Concurrent scanning**: Multiple threads handle packet I/O for optimal throughput
- **Memory efficiency**: Zero-copy packet processing where possible
- **Throttle control**: Configurable per-packet send delay via `throttle` builder
  field (default `200µs`); higher values reduce packet loss at the cost of scan speed
- **Timeout optimization**: Adaptive timeouts based on network response times

## Security Considerations

- **Requires root privileges** for raw socket access on Unix-like systems
- **Network scanning may be restricted** by network policies and firewalls
- **Built-in rate limiting** prevents network congestion and reduces detection risk
- **Minimal network footprint**: Optimized packet sizes and timing
- **Memory safety**: Rust's ownership system prevents buffer overflows and memory corruption
- **Use responsibly** and only on networks you own or have permission to scan
- **Logging**: All scan activities can be logged for audit purposes

### Ethical Usage Guidelines

- Always obtain proper authorization before scanning
- Respect network resources and avoid aggressive scanning
- Be aware that scanning activities may be logged by network security systems
- Consider the impact on network performance during large-scale scans

## Error Handling

The library uses `RLanLibError` for comprehensive error reporting:

## License

This project is dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Related Projects

- [go-lanscan](https://github.com/robgonnella/go-lanscan) - Original Go implementation
- [r-lancli](../cli/README.md) - Command-line interface using this library
- [r-lanterm](../term/README.md) - Terminal UI application using this library
