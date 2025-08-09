# R-LANTERM

Terminal GUI for discovering and managing on premise LAN devices.

Features:
- Save global ssh configuration used for all devices
- Save device specific ssh config that overrides global configuration
- Drop to ssh for any device found on network (requires ssh client to be installed)
- Run `traceroute` for device found on network (requires traceroute to be installed)
- Open terminal web browser on any port for any device found on network (requires lynx browser to be installed)

# Install

## Build from source

```bash
cargo build
../target/debug/r-lanterm --help

cargo build --release
../target/release/r-lanterm --help
```

## Run from source

```bash
cargo run -- --help
```
