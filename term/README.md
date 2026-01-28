# r-lanterm

A full-featured terminal user interface (TUI) application for managing and
interacting with networked LAN devices. This is the new and improved Rust
version of [ops](https://github.com/robgonnella/ops), providing an interactive
way to discover, monitor, and connect to devices on your local network.

## Features

- **Interactive Device Discovery**: Real-time ARP and SYN scanning with live
  updates
- **Device Management**: View detailed information about discovered network
  devices
- **SSH Integration**: Connect to devices via SSH with configurable credentials
- **Network Diagnostics**: Run traceroute commands directly from the interface
- **Web Browsing**: Open web browser on any port for any device (supports
  system default browser or lynx)
- **Persistent Configuration**: Save global and device-specific SSH
  configurations
- **Customizable Themes**: Multiple color themes (Blue, Emerald, Indigo, Red)
- **Activity Logs**: Built-in logs view for monitoring scan activity and
  debugging
- **Port Scanning**: Configurable port ranges for comprehensive network
  analysis
- **Vendor Detection**: MAC address vendor lookup for device identification
- **Hostname Resolution**: Automatic DNS resolution for discovered devices

## Prerequisites

- **Root privileges required**: Network scanning requires raw socket access
- **Rust 1.89.0+** - Install via [rustup.rs](https://rustup.rs/)
- **External tools** (optional but recommended):
  - `ssh` - For SSH connections to devices
  - `traceroute` - For network path analysis
  - `lynx` - For in-terminal web browsing (system browser works without this)

## Installation

### Building from Source

```bash
# Clone the repository
git clone https://github.com/robgonnella/r-lanscan
cd r-lanscan

# Build the terminal application
cargo build --release -p r-lanterm

# The binary will be available at ./target/release/r-lanterm
```

## Quick Start

### Basic Usage

Launch the application with default settings:

```bash
sudo r-lanterm
```

The application will automatically:

1. Detect your default network interface
2. Start scanning your local network (CIDR block)
3. Discover devices using ARP scanning
4. Perform SYN scanning on discovered devices
5. Display results in an interactive table

### Debug Mode

For troubleshooting or to see detailed logs:

```bash
sudo r-lanterm --debug
```

Debug mode disables the UI and shows detailed logging information.

## Command Line Options

### `--ports, -p <PORTS>`

Comma-separated list of ports and port ranges to scan.

**Default**: `22,80,443,2000-9999,27017`

**Examples**:

```bash
# Common ports only
sudo r-lanterm --ports 22,80,443

# Extended range
sudo r-lanterm --ports 1-1000,8000-9000

# Mixed specification
sudo r-lanterm --ports 22,80,443,8080,8443,3000-4000
```

### `--debug, -d`

Run in debug mode - prints logs instead of showing the UI.

**Use case**: Troubleshooting network issues, debugging scan problems.

```bash
sudo r-lanterm --debug
```

## User Interface

### Main Views

The application provides several interactive views:

#### 1. Devices View (Default)

- **Purpose**: Display all discovered network devices in a table
- **Columns**: IP Address, Hostname, Vendor, MAC Address, Open Ports
- **Navigation**: Use arrow keys or `j`/`k` to navigate
- **Selection**: Press `Enter` to view device details

#### 2. Device Detail View

- **Purpose**: Detailed information and actions for a specific device
- **Features**: SSH connection, traceroute, port browsing, device
  configuration

#### 3. Configuration View

- **Purpose**: Manage global and device-specific settings
- **Options**: SSH credentials, port ranges, themes

#### 4. Logs View

- **Purpose**: View real-time application logs and scan activity
- **Features**: Scrollable log history, displays scan events and status messages
- **Navigation**: Use arrow keys or `j`/`k` to scroll, mouse wheel supported

### Navigation and Controls

#### Global Controls

- **`q`** - Quit application
- **`Ctrl+C`** - Force quit (also handles external commands)
- **`v`** - Change view / Open view selection menu
- **`Esc`** - Cancel current action or go back

#### Devices View

- **`j` / `↓`** - Move down in device list
- **`k` / `↑`** - Move up in device list
- **`Enter`** - View selected device details
- **Mouse wheel** - Scroll through device list

#### Device Detail View

- **`Esc`** - Back to devices list
- **`c`** - Configure device-specific SSH settings
- **`s`** - Connect to device via SSH
- **`t`** - Run traceroute to device
- **`b`** - Browse device via web browser (specify port)

#### Configuration View

- **`c`** - Start/enter configuration mode
- **`Tab`** - Focus next input field
- **`Shift+Tab`** - Focus previous input field
- **`Enter`** - Save configuration
- **`Esc`** - Exit configuration mode
- **`← / →`** - Navigate theme colors (when in theme field)
- **`Backspace`** - Delete character in input fields

#### Logs View

- **`j` / `↓`** - Scroll down
- **`k` / `↑`** - Scroll up
- **Mouse wheel** - Scroll through logs

## Configuration

### Configuration Files

r-lanterm stores configuration in platform-specific directories:

- **Linux**: `~/.config/r-lanterm/config.yml`
- **macOS**: `~/Library/Application Support/r-lanterm/config.yml`
- **Windows**: `%APPDATA%/r-lanterm/config.yml`

### Global Configuration

The application maintains a global configuration per network (CIDR block):

```yaml
id: "home-network"
cidr: "192.168.1.0/24"
theme: "Blue"
ports: ["22", "80", "443", "2000-9999", "27017"]
default_ssh_user: "username"
default_ssh_port: "22"
default_ssh_identity: "/home/username/.ssh/id_rsa"
device_configs: {}
```

#### Configuration Options

- **`theme`**: Visual theme (Blue, Emerald, Indigo, Red)
- **`ports`**: Default ports to scan for all devices
- **`default_ssh_user`**: Default username for SSH connections
- **`default_ssh_port`**: Default SSH port
- **`default_ssh_identity`**: Path to SSH private key file

### Device-Specific Configuration

Override global settings for individual devices:

```yaml
device_configs:
  "aa:bb:cc:dd:ee:ff": # MAC address as key
    id: "router"
    ssh_port: 2222
    ssh_identity_file: "/home/user/.ssh/router_key"
    ssh_user: "admin"
```

### Themes

Available color themes:

- **Blue** (default): Classic blue color scheme
- **Emerald**: Green-based color palette
- **Indigo**: Purple/indigo color scheme
- **Red**: Red-based color scheme

Themes automatically adapt to terminal capabilities (true color vs basic
colors).

## Features in Detail

### Automatic Network Monitoring

r-lanterm continuously monitors your network:

1. **Initial Scan**: Comprehensive ARP + SYN scan on startup
2. **Periodic Updates**: Rescans every 15 seconds to detect changes
3. **Real-time Updates**: Live display of scan progress and results
4. **Device Tracking**: Maintains device information across scans

### SSH Integration

**Prerequisites**: SSH client must be installed on your system

**Features**:

- Uses configured SSH credentials (global or device-specific)
- Supports custom SSH ports and identity files
- Seamless transition - UI pauses while SSH session is active
- Returns to UI when SSH session ends

**Configuration**:

1. Navigate to Configuration view (`v` → select Config)
2. Press `c` to enter configuration mode
3. Set SSH user, port, and identity file path
4. Press `Enter` to save

### Traceroute Analysis

**Prerequisites**: `traceroute` command must be installed

**Features**:

- Shows network path to selected device
- Displays hop-by-hop latency information
- Results shown directly in the device detail view
- Uses ICMP traceroute for accurate results

**Usage**:

1. Select device in Devices view (`Enter`)
2. Press `t` in Device Detail view
3. Results appear in real-time

### Web Browsing

**Prerequisites**: `lynx` terminal browser (optional, for in-terminal browsing)

**Features**:

- Browse web interfaces on any device port
- Choose between system default browser or lynx (terminal-based)
- Custom port specification
- Useful for router admin interfaces, web servers, etc.

**Usage**:

1. Select device in Device Detail view
2. Press `b` to browse
3. Select browser type (default or lynx) using arrow keys
4. Enter port number
5. Press `Enter` to launch browser

### Port Scanning

**Configurable Ranges**:

- Default: `22,80,443,2000-9999,27017`
- Supports individual ports: `22,80,443`
- Supports port ranges: `8000-9000`
- Mixed specifications: `22,80,8000-9000`

**Scan Process**:

1. ARP scan discovers active devices
2. SYN scan checks configured ports on each device
3. Results update in real-time as ports are discovered
4. Open ports displayed in device table

## Troubleshooting

### Permission Issues

**Error**: `permission denied: must run with root privileges`

**Solution**: Run with sudo:

```bash
sudo r-lanterm
```

### No Devices Found

**Possible Causes**:

- Network interface detection issues
- Firewall blocking scan packets
- Devices configured to ignore ARP requests

**Solutions**:

1. **Check network connectivity**:

   ```bash
   # Verify your network configuration
   ip route show  # Linux
   route -n get default  # macOS
   ```

2. **Use debug mode**:

   ```bash
   sudo r-lanterm --debug
   ```

3. **Verify network interface**:
   - Application auto-detects default interface
   - Check that your network interface is active and has an IP

### SSH Connection Issues

**Common Problems**:

- SSH key permissions
- Incorrect SSH port or username
- SSH service not running on target device

**Solutions**:

1. **Verify SSH key permissions**:

   ```bash
   chmod 600 ~/.ssh/id_rsa
   chmod 644 ~/.ssh/id_rsa.pub
   ```

2. **Test SSH manually**:

   ```bash
   ssh -i ~/.ssh/id_rsa user@device_ip -p 22
   ```

3. **Check device SSH configuration**:
   - Ensure SSH daemon is running
   - Verify correct port and authentication settings

### External Command Issues

**Missing Commands**:

- `ssh`: Install OpenSSH client
- `traceroute`: Install traceroute package
- `lynx`: Install lynx web browser

**Installation Examples**:

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install openssh-client traceroute lynx

# macOS (using Homebrew)
brew install openssh
brew install traceroute
brew install lynx

# CentOS/RHEL/Fedora
sudo yum install openssh-clients traceroute lynx
# or for newer versions:
sudo dnf install openssh-clients traceroute lynx
```

### Performance Issues

**Large Networks**:

- Reduce port scan range: `--ports 22,80,443`
- Network scans are performed every 15 seconds
- UI rendering optimized for real-time updates

**Slow Terminal**:

- Try different themes for better performance
- Reduce terminal window size if rendering is slow
- Use debug mode to check for underlying issues

## Use Cases

### Network Administration

- **Device Discovery**: Find all active devices on your network
- **Port Auditing**: Identify open services across all devices
- **SSH Management**: Quick access to multiple servers
- **Network Monitoring**: Continuous monitoring of network changes

### Security Analysis

- **Port Scanning**: Identify potentially vulnerable services
- **Device Inventory**: Track all devices connecting to network
- **Service Detection**: Monitor for unauthorized services
- **Network Mapping**: Understand network topology with traceroute

### Development and Testing

- **Service Testing**: Check if development servers are accessible
- **Network Debugging**: Use traceroute for connectivity issues
- **Web Interface Access**: Browse device web interfaces with lynx
- **SSH Automation**: Quick SSH access to development machines

## Integration

### Automation Scripts

While r-lanterm is primarily interactive, it can be integrated into workflows:

```bash
# Run in debug mode for logging
sudo r-lanterm --debug 2>&1 | tee network-scan.log

# Custom port scanning for specific services
sudo r-lanterm --ports 22,3389,5900  # SSH, RDP, VNC
```

### Configuration Management

Configuration files can be version controlled or shared:

```bash
# Backup configuration
cp ~/.config/r-lanterm/config.yml ~/backup/

# Share configuration across systems
scp config.yml user@remote:~/.config/r-lanterm/
```

## Security Considerations

### Network Impact

- **Scanning Traffic**: ARP and SYN scans generate network traffic
- **Detection**: Network monitoring systems may detect scanning activity
- **Rate Limiting**: Built-in delays prevent network congestion
- **Responsible Use**: Only scan networks you own or have permission to test

### SSH Security

- **Key Management**: Use secure SSH key pairs
- **Key Permissions**: Ensure proper file permissions on SSH keys
- **Connection Logging**: SSH connections may be logged by target systems
- **Authentication**: Prefer key-based authentication over passwords

### Data Storage

- **Configuration**: SSH credentials stored in configuration files
- **File Permissions**: Ensure config directory has appropriate permissions
- **Sensitive Data**: Consider encrypting configuration files if needed

## License

This project is dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Related Tools

- **[r-lanlib](../lib/README.md)**: The underlying Rust library powering this
  application
- **[r-lancli](../cli/README.md)**: Command-line interface for batch scanning

## Changelog

See [CHANGELOG.md](./CHANGELOG.md) for version history.
