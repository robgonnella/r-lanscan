# Contributing to r-lanscan

## Quick Start

```bash
# Build all crates
cargo build

# Run tests
just test        # or: cargo test

# Lint
just lint        # or: cargo clippy --all-targets --all-features

# Run with sudo (required for raw sockets)
just scan        # CLI
just term        # Terminal UI
```

## Project Structure

```
r-lanscan/
├── lib/    # r-lanlib - Core scanning library
├── cli/    # r-lancli - Command-line interface
└── term/   # r-lanterm - Terminal UI (ratatui)
```

## Architecture

### Scanner Pattern (lib)

All scanners implement the `Scanner` trait and communicate via channels:

```rust
pub trait Scanner: Sync + Send {
    fn scan(&self) -> JoinHandle<Result<()>>;
}
```

Results are sent as `ScanMessage` variants (`ARPScanResult`, `SYNScanResult`,
`Done`).

### Packet I/O Abstraction (lib)

`Reader` and `Sender` traits in `lib/src/packet/wire.rs` abstract packet
operations. This enables unit testing with mocks instead of real network
interfaces.

### Terminal UI State (term)

Redux-like architecture in `term/src/ui/store/`:

- **State** - Single source of truth
- **Action** - Describes state changes
- **Reducer** - Pure function: `(State, Action) -> State`
- **Derived** - Computed selectors

## Conventions

### Module Structure

Use `module.rs` + `module/` directory pattern. **Do not use `mod.rs`**.

```
src/
├── foo.rs           # Module declaration
└── foo/             # Submodules
    └── bar.rs
```

### Tests

Test files use `*_tests.rs` co-located with source:

```
src/
├── scanner.rs
└── scanner_tests.rs
```

## Requirements

- Rust 1.89.0+
- Root/admin privileges for running scans
- `just` task runner (optional but recommended)
