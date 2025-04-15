ARP and SYN network scanning library, cli, and ui written in rust! This is the
rust version of [go-lanscan]

**Table of contents**
- [Libs](#libs)
- [Bin](#bin)
- [Usage](#usage)
- [Testing](#testing)

### Libs

- [r-lanlib](./r-lanlib/README.md)

### Bin

- [r-lanscan](./r-lanscan/README.md)
- [r-lanui](./r-lanui/README.md)

### Testing

Prerequisites

- install llvm-cov

```zsh
cargo +stable install cargo-llvm-cov --locked
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
