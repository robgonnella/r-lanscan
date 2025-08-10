# r-lanlib

A Rust library crate for performing network scanning operations on any local area network (LAN). This is the Rust version of the [go-lanscan package](https://github.com/robgonnella/go-lanscan).

## Features

- **ARP Scanning**: Discover devices on your network using Address Resolution Protocol
- **SYN Scanning**: Detect open ports on discovered devices using TCP SYN packets
- **Full Scanning**: Combined ARP and SYN scanning in a single operation
- **Vendor Detection**: Identify device manufacturers using MAC address lookup
- **Hostname Resolution**: Resolve hostnames for discovered devices
- **Async Communication**: Channel-based communication for real-time scan results
- **Flexible Targeting**: Support for CIDR blocks, IP ranges, and port ranges

## Requirements

- **Root privileges required**: This library performs raw packet operations that require elevated permissions
- **Rust 1.89.0+** with 2024 edition support
- **System dependencies**: `libssl-dev` (Linux), `openssl` (macOS) for cryptographic operations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
r-lanlib = "0.1.0"
```

## Quick Start

### Basic ARP Scanning

```rust
use std::{sync::mpsc, time::Duration};
use r_lanlib::{
    network, packet,
    scanners::{
        arp_scanner::{ARPScanner, ARPScannerArgs},
        Device, ScanMessage, Scanner,
    },
    targets::ips::IPTargets,
};

// Get default network interface
let interface = network::get_default_interface()
    .expect("Cannot find network interface");

// Create packet wire (reader/sender)
let wire = packet::wire::default(&interface)
    .expect("Failed to create packet wire");

// Define IP targets (scan entire subnet)
let ip_targets = IPTargets::new(vec![interface.cidr.clone()]);

// Create communication channel
let (tx, rx) = mpsc::channel::<ScanMessage>();

// Configure scanner
let scanner = ARPScanner::new(ARPScannerArgs {
    interface: &interface,
    packet_reader: wire.0,
    packet_sender: wire.1,
    targets: ip_targets,
    source_port: 54321,
    include_vendor: true,
    include_host_names: true,
    idle_timeout: Duration::from_millis(10000),
    notifier: tx,
});

// Start scanning (runs in background thread)
let handle = scanner.scan();

// Process results in real-time
let mut devices = Vec::new();
loop {
    match rx.recv().expect("Failed to receive message") {
        ScanMessage::Done => break,
        ScanMessage::ARPScanResult(device) => {
            println!("Found device: {} ({})", device.ip, device.hostname);
            devices.push(device);
        }
        ScanMessage::Info(scanning) => {
            println!("Scanning: {}", scanning.ip);
        }
    }
}

handle.join().expect("Scanner thread failed");
println!("Discovered {} devices", devices.len());
```

### SYN Port Scanning

```rust
use r_lanlib::{
    scanners::{
        syn_scanner::{SYNScanner, SYNScannerArgs},
        Device, SYNScanResult, Scanner,
    },
    targets::ports::PortTargets,
};

// Define target devices (from previous ARP scan or manual)
let devices = vec![
    Device {
        hostname: "router".to_string(),
        ip: "192.168.1.1".to_string(),
        mac: "aa:bb:cc:dd:ee:ff".to_string(),
        vendor: "".to_string(),
        is_current_host: false,
    }
];

// Define port targets
let port_targets = PortTargets::new(vec![
    "22".to_string(),      // SSH
    "80".to_string(),      // HTTP
    "443".to_string(),     // HTTPS
    "8000-9000".to_string(), // Port range
]);

let scanner = SYNScanner::new(SYNScannerArgs {
    interface: &interface,
    packet_reader: wire.0,
    packet_sender: wire.1,
    targets: devices,
    ports: port_targets,
    source_port: 54321,
    idle_timeout: Duration::from_millis(10000),
    notifier: tx,
});

// Process SYN scan results
let mut results = Vec::new();
let handle = scanner.scan();

loop {
    match rx.recv().expect("Failed to receive message") {
        ScanMessage::Done => break,
        ScanMessage::SYNScanResult(result) => {
            println!("Open port {} on {}",
                result.open_port.id,
                result.device.ip
            );
            results.push(result);
        }
        _ => {}
    }
}
```

### Full Scanning (ARP + SYN)

```rust
use r_lanlib::scanners::{
    full_scanner::{FullScanner, FullScannerArgs},
    Scanner,
};

let scanner = FullScanner::new(FullScannerArgs {
    interface: &interface,
    packet_reader: wire.0,
    packet_sender: wire.1,
    targets: ip_targets,
    ports: port_targets,
    include_vendor: true,
    include_host_names: true,
    idle_timeout: Duration::from_millis(10000),
    notifier: tx,
    source_port: 54321,
});

// This will perform ARP discovery first, then SYN scan on found devices
let handle = scanner.scan();
```

## API Reference

### Core Modules

#### `network`
Provides helpers for selecting network interfaces:
- `get_default_interface()` - Get the default network interface
- `get_interface(name)` - Get a specific interface by name
- `get_available_port()` - Find an available port for scanning

#### `packet`
Low-level packet creation and transmission:
- `wire::default(interface)` - Create default packet reader/sender pair
- Various packet builders for ARP, SYN, RST packets

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
    pub ip: String,
    pub mac: String,
    pub vendor: String,
    pub is_current_host: bool,
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

#### `SYNScanResult`
Result of a SYN scan operation:
```rust
pub struct SYNScanResult {
    pub device: Device,
    pub open_port: Port,
}
```

#### `ScanMessage`
Messages sent over the notification channel:
```rust
pub enum ScanMessage {
    Done,                           // Scanning complete
    Info(Scanning),                 // Status update
    ARPScanResult(Device),          // ARP discovery result
    SYNScanResult(SYNScanResult),   // SYN scan result
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

## Examples

The library includes several complete examples in the `examples/` directory:

- **`arp-scanner.rs`** - Basic ARP device discovery
- **`syn-scanner.rs`** - Port scanning on known devices
- **`full-scanner.rs`** - Complete network reconnaissance

Run examples from the workspace root with:
```bash
sudo -E cargo run --example arp-scanner -p r-lanlib
sudo -E cargo run --example syn-scanner -p r-lanlib
sudo -E cargo run --example full-scanner -p r-lanlib
```

## Configuration Options

### Scanner Timeouts
- `idle_timeout` - How long to wait for responses before concluding scan
- Default: 10 seconds (10,000ms)
- Recommended: 5-30 seconds depending on network size and latency

### Scanner Features
- `include_vendor` - Perform MAC address vendor lookup using IEEE OUI database
- `include_host_names` - Resolve hostnames via reverse DNS lookup
- `source_port` - Source port for scan packets (auto-selected if not specified)

### Performance Tuning
- **Concurrent scanning**: Multiple threads handle packet I/O for optimal throughput
- **Memory efficiency**: Zero-copy packet processing where possible
- **Network-aware**: Automatic rate limiting to prevent network congestion
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

The library uses `ScanError` for comprehensive error reporting:

```rust
pub struct ScanError {
    pub ip: Option<String>,
    pub port: Option<String>,
    pub error: Box<dyn Error>,
}
```

All scanner operations return `Result<(), ScanError>` for proper error handling.

## Dependencies

- `pnet` - Low-level networking primitives and packet crafting
- `ipnet` - IP network utilities and CIDR block handling
- `oui-data` - IEEE OUI database for MAC address vendor lookup
- `dns-lookup` - Hostname resolution and reverse DNS
- `serde` - Serialization/deserialization with derive support
- `log` - Structured logging interface
- `paris` - Enhanced logging with timestamps and colors
- `simplelog` - Simple logging implementation with paris integration

### Development Dependencies
- `mockall` ^0.13 - Mock object generation for testing

## License

This project uses the same license as the parent r-lanscan project.

## Contributing

See the main project [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

## Related Projects

- [go-lanscan](https://github.com/robgonnella/go-lanscan) - Original Go implementation
- [r-lancli](../cli/README.md) - Command-line interface using this library
- [r-lanterm](../term/README.md) - Terminal UI application using this library
