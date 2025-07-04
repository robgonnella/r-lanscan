ARP and SYN network scanning library, cli, and ui written in rust! This is the
rust version of [go-lanscan]

**Table of contents**
- [Libs](#libs)
- [Bin](#bin)
- [Testing](#testing)

### Libs

- [r-lanlib](./r-lanlib/README.md)

### Bin

- [r-lancli](./r-lancli/README.md)
- [r-lanterm](./r-lanterm/README.md)

### Testing

Prerequisites

- install llvm-cov
- install cargo-insta

```zsh
cargo +stable install cargo-llvm-cov --locked
cargo install cargo-insta
```

Run tests

```zsh
# Run all tests
cargo test

# Run all tests and print coverage
cargo llvm-cov

# Run all tests and generate html report
cargo llvm-cov --html

# Run tests for specific project
cargo test -p r-lanlib
```

[go-lanscan]: https://github.com/robgonnella/go-lanscan
