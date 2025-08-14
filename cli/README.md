# r-lancli

A command-line interface for performing network scanning operations on local area networks (LANs). This CLI tool uses the [r-lanlib](../lib/README.md) library to provide comprehensive network reconnaissance capabilities.

## Features

- **ARP Scanning**: Discover active devices on your network using Address Resolution Protocol
- **SYN Port Scanning**: Detect open ports on discovered devices using TCP SYN packets
- **Flexible Target Specification**: Support for individual IPs, IP ranges, and CIDR blocks
- **Port Range Scanning**: Scan specific ports or port ranges
- **Device Information**: Optional MAC address vendor lookup and hostname resolution
- **Multiple Output Formats**: Human-readable tables or JSON for programmatic use
- **Network Interface Selection**: Choose specific network interfaces for scanning
- **Configurable Timeouts**: Adjust scan timing for different network conditions

## Installation

### Prerequisites

- **Root privileges required**: Network scanning requires raw socket access
- **Rust 1.89.0+** - Install via [rustup.rs](https://rustup.rs/)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/robgonnella/r-lanscan
cd r-lanscan

# Build the CLI tool
cargo build --release -p r-lancli

# The binary will be available at ./target/release/r-lancli
```

## Quick Start

### Basic Network Scan

Scan your entire local network (discovers devices and scans all ports):

```bash
sudo r-lancli
```

### ARP-Only Device Discovery

Quickly discover devices without port scanning:

```bash
sudo r-lancli --arp-only --vendor --host-names
```

### Scan Specific Targets

Scan specific IP addresses or ranges:

```bash
sudo r-lancli --targets 192.168.1.1,192.168.1.10-20,10.0.0.0/24
```

### Port-Specific Scanning

Scan only common ports:

```bash
sudo r-lancli --ports 22,80,443,8080,8443
```

### JSON Output

Get results in JSON format for scripting:

```bash
sudo r-lancli --json --quiet > scan_results.json
```

## Command Line Options

### Target Specification

#### `--targets, -t <TARGETS>`

Comma-separated list of scan targets. Supports:

- **Individual IPs**: `192.168.1.1`
- **IP ranges**: `192.168.1.1-192.168.1.100`
- **CIDR blocks**: `192.168.1.0/24`, `10.0.0.0/16`

**Default**: Uses the CIDR block of the selected network interface

**Examples**:

```bash
# Single IP
sudo r-lancli --targets 192.168.1.1

# Multiple IPs
sudo r-lancli --targets 192.168.1.1,192.168.1.5,192.168.1.10

# IP range
sudo r-lancli --targets 192.168.1.1-192.168.1.50

# CIDR block
sudo r-lancli --targets 192.168.1.0/24

# Mixed specification
sudo r-lancli --targets 192.168.1.1,192.168.1.10-20,10.0.0.0/24
```

#### `--ports, -p <PORTS>`

Comma-separated list of ports and port ranges to scan.

**Default**: `1-65535` (all ports)

**Examples**:

```bash
# Common ports
sudo r-lancli --ports 22,80,443

# Port ranges
sudo r-lancli --ports 1-1000,8000-9000

# Mixed specification
sudo r-lancli --ports 22,80,443,8000-9000,3389
```

### Scan Options

#### `--arp-only`

Perform only ARP scanning, skipping SYN port scanning.

**Use case**: Quick device discovery without the time overhead of port scanning.

```bash
sudo r-lancli --arp-only --vendor --host-names
```

#### `--vendor`

Enable MAC address vendor lookup to identify device manufacturers.

```bash
sudo r-lancli --vendor
```

#### `--host-names`

Enable reverse DNS lookup to resolve hostnames for discovered devices.

```bash
sudo r-lancli --host-names
```

### Network Configuration

#### `--interface, -i <INTERFACE>`

Select a specific network interface for scanning.

**Default**: Automatically selects the default network interface

**Examples**:

```bash
# Use specific interface
sudo r-lancli --interface eth0

# List available interfaces (use system tools)
ip link show  # Linux
ifconfig      # macOS/BSD
```

#### `--source-port <SOURCE_PORT>`

Set the source port for outgoing scan packets.

**Default**: Automatically selects an available port

```bash
sudo r-lancli --source-port 12345
```

### Output Options

#### `--json`

Output results in JSON format instead of human-readable tables.

**Use case**: Programmatic processing, integration with other tools.

```bash
sudo r-lancli --json > results.json
```

#### `--quiet, -q`

Suppress progress messages, only show final results.

**Use case**: Cleaner output for scripting and automation.

```bash
sudo r-lancli --quiet --json
```

### Timing and Performance

#### `--idle-timeout-ms <MILLISECONDS>`

Set the idle timeout for scan operations.

**Default**: `10000` (10 seconds)

**Use case**: Adjust for slower networks or more thorough scanning.

```bash
# Faster scanning (less thorough)
sudo r-lancli --idle-timeout-ms 5000

# Slower scanning (more thorough)
sudo r-lancli --idle-timeout-ms 30000
```

### Debugging

#### `--debug`

Enable debug logging for troubleshooting scan operations.

```bash
sudo r-lancli --debug
```

## Output Formats

### Table Format (Default)

**ARP Results**:

```
+---------------+----------+-------------------+------------------------+
| IP            | HOSTNAME | MAC               | VENDOR                 |
+---------------+----------+-------------------+------------------------+
| 192.168.1.1   | router   | aa:bb:cc:dd:ee:ff | Netgear                |
| 192.168.1.100 | laptop   | 11:22:33:44:55:66 | Apple, Inc.            |
| 192.168.1.150 |          | 99:88:77:66:55:44 | Samsung Electronics    |
+---------------+----------+-------------------+------------------------+
```

**SYN Results** (with port scanning):

```
+---------------+----------+-------------------+------------------------+-------------+
| IP            | HOSTNAME | MAC               | VENDOR                 | OPEN_PORTS  |
+---------------+----------+-------------------+------------------------+-------------+
| 192.168.1.1   | router   | aa:bb:cc:dd:ee:ff | Netgear                | 22, 80, 443 |
| 192.168.1.100 | laptop   | 11:22:33:44:55:66 | Apple, Inc.            | 22, 5900    |
+---------------+----------+-------------------+------------------------+-------------+
```

### JSON Format

```json
[
  {
    "hostname": "router",
    "ip": "192.168.1.1",
    "mac": "aa:bb:cc:dd:ee:ff",
    "vendor": "Netgear",
    "is_current_host": false,
    "open_ports": [
      { "id": 22, "service": "ssh" },
      { "id": 80, "service": "http" },
      { "id": 443, "service": "https" }
    ]
  }
]
```

## Common Use Cases

### Network Discovery

Discover all devices on your local network:

```bash
sudo r-lancli --arp-only --vendor --host-names
```

### Security Auditing

Comprehensive scan with detailed information:

```bash
sudo r-lancli --vendor --host-names --json > network_audit.json
```

### Service Discovery

Find devices running specific services:

```bash
sudo r-lancli --ports 22,80,443,3389,5900 --vendor
```

### Quick Port Check

Check if specific hosts have certain ports open:

```bash
sudo r-lancli --targets 192.168.1.1,192.168.1.100 --ports 22,80,443
```

### Subnet Scanning

Scan multiple subnets:

```bash
sudo r-lancli --targets 192.168.1.0/24,192.168.2.0/24 --arp-only
```

## Integration and Automation

### Bash Scripting

```bash
#!/bin/bash

# Perform scan and save results
sudo r-lancli --json --quiet > network_scan.json

# Process results with jq
jq '.[] | select(.open_ports | length > 0)' network_scan.json > devices_with_ports.json

echo "Found $(jq 'length' devices_with_ports.json) devices with open ports"
```

### Python Integration

```python
import json
import subprocess

# Run scan
result = subprocess.run([
    'sudo', 'r-lancli',
    '--json', '--quiet',
    '--targets', '192.168.1.0/24'
], capture_output=True, text=True)

# Parse results
devices = json.loads(result.stdout)

for device in devices:
    print(f"Device: {device['ip']} ({device['hostname']})")
    if 'open_ports' in device:
        ports = [str(port['id']) for port in device['open_ports']]
        print(f"  Open ports: {', '.join(ports)}")
```

### Cron Jobs

```bash
# Add to crontab for periodic scanning
# Run every hour and log changes
0 * * * * /usr/local/bin/r-lancli --json --quiet > /var/log/network-scan-$(date +\%Y\%m\%d-\%H).json 2>&1
```

## Troubleshooting

### Permission Errors

**Error**: `permission denied: must run with root privileges`

**Solution**: Run with `sudo`:

```bash
sudo r-lancli
```

### Network Interface Issues

**Error**: `cannot find interface`

**Solutions**:

1. List available interfaces:

   ```bash
   # Linux
   ip link show

   # macOS/BSD
   ifconfig
   ```

2. Specify interface explicitly:
   ```bash
   sudo r-lancli --interface eth0
   ```

### No Results Found

**Possible causes**:

- Firewall blocking scan packets
- Network devices configured to ignore ARP/ICMP
- Incorrect network range specified

**Solutions**:

1. Increase timeout:

   ```bash
   sudo r-lancli --idle-timeout-ms 30000
   ```

2. Use debug mode:

   ```bash
   sudo r-lancli --debug
   ```

3. Verify network configuration:
   ```bash
   # Check your IP and network
   ip route show  # Linux
   route -n get default  # macOS
   ```

### Performance Issues

**For large networks**:

1. Use ARP-only for initial discovery:

   ```bash
   sudo r-lancli --arp-only
   ```

2. Limit port ranges:

   ```bash
   sudo r-lancli --ports 1-1000
   ```

3. Scan smaller subnets:
   ```bash
   sudo r-lancli --targets 192.168.1.0/26
   ```

## Security and Ethics

### Responsible Usage

- **Only scan networks you own or have explicit permission to test**
- **Be aware that network scanning may be logged by security systems**
- **Some networks may consider scanning as hostile activity**
- **Consider network impact - scanning can generate significant traffic**

### Legal Considerations

- Network scanning may be restricted by local laws and regulations
- Corporate networks often have policies against unauthorized scanning
- Always obtain proper authorization before scanning

## License

This project is dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Related Tools

- **[r-lanlib](../lib/README.md)**: The underlying Rust library powering this CLI
- **[r-lanterm](../term/README.md)**: Terminal UI application for interactive network management
