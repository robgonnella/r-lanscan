# r-lanscan

ARP and SYN network scanning library, cli, and ui written in rust! This is the
rust version of [go-lanscan]

**Table of contents**
- [Libs](#libs)
- [Bin](#bin)
- [Testing](#testing)

## Libs

- [r-lanlib](./lib/README.md): provides access to arp and syn scanning
  tools to allow you to build your own network scanning tools

## Bin

- [r-lancli](./cli/README.md): cli for scanning LAN in terminal
- [r-lanterm](./term/README.md): terminal UI for discovering and managing
  devices on LAN

## Testing

**Prerequisites**

- install llvm-cov
- install cargo-insta
- install just

```zsh
cargo +stable install cargo-llvm-cov --locked
cargo install cargo-insta
cargo install just
```

**Run tests**

```zsh
# Run all tests
just test

# Run all tests and print coverage
just test-report

# Run all tests and generate html report
just test-report --html

# Run tests for specific project
just test -p r-lanlib
```

[go-lanscan]: https://github.com/robgonnella/go-lanscan
[just]: https://just.systems/man/en/
