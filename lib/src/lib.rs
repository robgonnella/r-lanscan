//! Library package for performing network scanning of any LAN
//!
//! This is the rust version of [go-lanscan package](https://github.com/robgonnella/go-lanscan)
//!
//! # Examples
//!
//! ## ARP Scanning
//!
//! <https://github.com/robgonnella/r-lanscan/blob/main/r-lanlib/examples/arp-scanner.rs>
//!
//! ```bash
//! sudo -E cargo run --example arp-scanner
//! ```
//!
//! ## SYN Scanning
//!
//! <https://github.com/robgonnella/r-lanscan/blob/main/r-lanlib/examples/syn-scanner.rs>
//!
//! ```bash
//! sudo -E cargo run --example syn-scanner
//! ```
//!
//! ## Full Scanning (ARP + SYN)
//!
//! <https://github.com/robgonnella/r-lanscan/blob/main/r-lanlib/examples/full-scanner.rs>
//!
//! ```bash
//! sudo -E cargo run --example full-scanner
//! ```

#![deny(missing_docs)]
pub mod network;
pub mod packet;
pub mod scanners;
pub mod targets;
