# r-lanscan

[![Build Status](https://github.com/robgonnella/r-lanscan/actions/workflows/rust.yml/badge.svg)](https://github.com/robgonnella/r-lanscan/actions)

A comprehensive suite of network scanning tools for local area networks, written in Rust. This is the modern Rust implementation of the [go-lanscan](https://github.com/robgonnella/go-lanscan) project, offering improved performance, safety, and usability.

## Features

- 🔍 **ARP Scanning** - Discover active devices on your network
- 🔌 **SYN Port Scanning** - Detect open ports and services on discovered devices
- 🖥️ **Terminal UI** - Interactive interface for network management and device interaction
- 📚 **Library Crate** - Build custom network scanning applications
- 🚀 **High Performance** - Optimized Rust implementation with concurrent scanning
- 🛡️ **Memory Safe** - Rust's safety guarantees prevent common networking bugs

## Components

r-lanscan is organized as a Cargo workspace with three main components:

### 📚 [r-lanlib](./lib/README.md) - Network Scanning Library

The core library providing network scanning capabilities for building custom applications.

```rust
use r_lanlib::{network, packet, scanners::*};

// Discover devices on your network
let interface = network::get_default_interface()?;
let scanner = ARPScanner::new(/* ... */);
let devices = scanner.scan()?;
```

**Key Features:**
- ARP and SYN scanning implementations
- Flexible target specification (IPs, ranges, CIDR blocks)
- Real-time results via channels
- Vendor lookup and hostname resolution
- Cross-platform network interface detection

### 🖥️ [r-lancli](./cli/README.md) - Command Line Interface

A powerful CLI tool for network reconnaissance and analysis.

```bash
# Scan entire local network
sudo r-lancli

# Scan specific targets with custom ports
sudo r-lancli --targets 192.168.1.0/24 --ports 22,80,443,8080

# Export results as JSON
sudo r-lancli --json --quiet > scan_results.json
```

**Key Features:**
- Comprehensive network scanning with customizable options
- Human-readable tables and JSON output
- Flexible target and port specification
- Vendor lookup and hostname resolution
- Integration-friendly for scripting and automation

### 🎮 [r-lanterm](./term/README.md) - Terminal UI Application

An interactive terminal user interface for network management and device interaction.

```bash
# Launch interactive terminal UI
sudo r-lanterm

# Customize port scanning
sudo r-lanterm --ports 22,80,443,3389,5900
```

**Key Features:**
- Real-time network monitoring with live updates
- SSH integration for direct device access
- Built-in traceroute and web browsing (lynx)
- Persistent configuration management
- Multiple color themes and customizable interface
- Device-specific and global SSH configurations

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/robgonnella/r-lanscan
cd r-lanscan

# Build all components
cargo build --release

# Or build individual components
cargo build --release -p r-lanlib
cargo build --release -p r-lancli
cargo build --release -p r-lanterm
```

### Using Justfile (Recommended)

```bash
# Install just command runner
cargo install just

# Scan your network with CLI
just scan --help
just scan --targets 192.168.1.0/24

# Launch terminal UI
just term --ports 22,80,443

# Run tests
just test

# Generate documentation
just readme
```

### Basic Usage Examples

**Quick Network Scan:**
```bash
sudo r-lancli --arp-only --vendor --host-names
```

**Comprehensive Port Analysis:**
```bash
sudo r-lancli --ports 1-1000 --json > network_audit.json
```

**Interactive Network Management:**
```bash
sudo r-lanterm  # Launch terminal UI for full interactive experience
```

## Requirements

- **Rust 1.85.1+** - Install via [rustup.rs](https://rustup.rs/)
- **Root/Administrator privileges** - Required for raw socket operations
- **Optional external tools** (for terminal UI):
  - `ssh` - For device connections
  - `traceroute` - For network path analysis
  - `lynx` - For terminal web browsing

## Documentation

- 📖 **[Library Documentation](./lib/README.md)** - API reference and examples for r-lanlib
- 💻 **[CLI Documentation](./cli/README.md)** - Complete command-line reference and usage examples
- 🖥️ **[Terminal UI Documentation](./term/README.md)** - Interactive interface guide and keyboard shortcuts

## Use Cases

### Network Administration
- Discover all devices on your network segments
- Monitor network changes and new device connections
- Audit open ports and services across your infrastructure
- Quick SSH access to multiple servers and devices

### Security Analysis
- Identify unauthorized devices on your network
- Detect unexpected open ports and services
- Map network topology and device relationships
- Monitor for security compliance across network segments

### Development and Testing
- Verify service availability during development
- Test network connectivity and firewall rules
- Debug network issues with integrated diagnostic tools
- Automate network discovery in CI/CD pipelines

## Development

### Building from Source

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Run with coverage reporting
cargo llvm-cov

# Lint code
cargo clippy --all-targets --all-features
```

### Project Structure

```
r-lanscan/
├── lib/          # Core scanning library (r-lanlib)
├── cli/          # Command-line interface (r-lancli)
├── term/         # Terminal UI application (r-lanterm)
├── book/         # Documentation and examples
└── Cargo.toml    # Workspace configuration
```

## Security and Ethics

⚠️ **Important:** This tool is designed for legitimate network administration and security analysis. Always ensure you have proper authorization before scanning networks.

- Only scan networks you own or have explicit permission to test
- Be aware that network scanning may trigger security monitoring systems
- Some jurisdictions have laws regulating network scanning activities
- Use responsibly and in accordance with your organization's security policies

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines on:

- Setting up the development environment
- Running tests and generating coverage reports
- Code style and formatting requirements
- Submitting pull requests

## License

This project is open source. Please see the LICENSE file for full license details.

## Related Projects

- [go-lanscan](https://github.com/robgonnella/go-lanscan) - Original Go implementation
- [ops](https://github.com/robgonnella/ops) - Original terminal UI concept
- [nmap](https://nmap.org/) - Comprehensive network discovery and security auditing
