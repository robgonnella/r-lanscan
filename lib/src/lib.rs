//! Library package for performing network scanning of any LAN
//!
//! This is the rust version of [go-lanscan package](https://github.com/robgonnella/go-lanscan)
//!
//! # Examples
//!
//! ## ARP Scanning
//!
//! <https://github.com/robgonnella/r-lanscan/blob/main/lib/examples/arp-scanner.rs>
//!
//! ```bash
//! sudo -E cargo run --example arp-scanner -p r-lanlib
//! ```
//!
//! ## SYN Scanning
//!
//! <https://github.com/robgonnella/r-lanscan/blob/main/lib/examples/syn-scanner.rs>
//!
//! ```bash
//! sudo -E cargo run --example syn-scanner -p r-lanlib
//! ```
//!
//! ## Full Scanning (ARP + SYN)
//!
//! <https://github.com/robgonnella/r-lanscan/blob/main/lib/examples/full-scanner.rs>
//!
//! ```bash
//! sudo -E cargo run --example full-scanner -p r-lanlib
//! ```

#![deny(missing_docs)]
pub mod network;
pub mod packet;
pub mod scanners;
pub mod targets;
