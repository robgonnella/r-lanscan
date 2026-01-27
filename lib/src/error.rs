//! Custom Error and Result types for this library

use std::{
    any::Any,
    num::ParseIntError,
    sync::{
        MutexGuard, PoisonError,
        mpsc::{RecvError, SendError},
    },
};
use thiserror::Error;

use crate::{
    packet::{
        Reader, Sender, arp_packet::ArpPacketBuilderError,
        heartbeat_packet::HeartbeatPacketBuilderError,
        rst_packet::RstPacketBuilderError, syn_packet::SynPacketBuilderError,
    },
    scanners::{
        ScanMessage, arp_scanner::ARPScannerBuilderError,
        heartbeat::HeartBeatBuilderError, syn_scanner::SYNScannerBuilderError,
    },
};

/// Custom Error type for this library
#[derive(Error, Debug)]
pub enum RLanLibError {
    /// Error coming directly off the wire
    #[error("wire error: {_0}")]
    Wire(String),

    /// Errors resulting from events channel
    #[error("failed to send notification message: {:#?}", _0)]
    NotifierSendError(#[from] SendError<Box<ScanMessage>>),

    /// Error obtaining lock on packet reader
    #[error("failed to get lock on packet reader: {_0}")]
    PacketReaderLock(String),

    /// Error obtaining lock on packet sender
    #[error("failed to get lock on packet sender: {_0}")]
    PacketSenderLock(String),

    /// Generic thread error
    #[error("thread error: {_0}")]
    ThreadError(String),

    /// Errors when consuming messages from channels
    #[error("failed to receive message from channel: {:#?}", _0)]
    ChannelReceive(#[from] RecvError),

    /// Error generated during ARP packet construction
    #[error("failed to build ARP packet: {_0}")]
    ArpPacketBuild(#[from] ArpPacketBuilderError),

    /// Error resulting from failure to build ARP scanner
    #[error("failed to build arp scanner: {_0}")]
    ArpScannerBuild(#[from] ARPScannerBuilderError),

    /// Error resulting from failure to build SYN scanner
    #[error("failed to build syn scanner: {_0}")]
    SynScannerBuild(#[from] SYNScannerBuilderError),

    /// Error resulting from failure to build Heartbeat
    #[error("failed to build heartbeat: {_0}")]
    HeartBeatBuild(#[from] HeartBeatBuilderError),

    /// Error generated during RST packet construction
    #[error("failed to build RST packet: {_0}")]
    RstPacketBuild(#[from] RstPacketBuilderError),

    /// Error generated during SYN packet construction
    #[error("failed to build SYN packet: {_0}")]
    SynPacketBuild(#[from] SynPacketBuilderError),

    /// Error generated during heartbeat packet construction
    #[error("failed to build heartbeat packet: {_0}")]
    HeartbeatPacketBuild(#[from] HeartbeatPacketBuilderError),

    /// Wrapping errors related to scanning
    #[error("scanning error: {error} - ip: {:#?}, port: {:#?}", ip, port)]
    Scan {
        /// The error message encountered
        error: String,
        /// The associated IP address being scanned
        ip: Option<String>,
        /// The associated port being scanned
        port: Option<String>,
    },
}

impl From<Box<dyn Any + Send>> for RLanLibError {
    fn from(value: Box<dyn Any + Send>) -> Self {
        if let Some(s) = value.downcast_ref::<&'static str>() {
            Self::ThreadError(format!("Thread panicked with: {}", s))
        } else if let Some(s) = value.downcast_ref::<String>() {
            Self::ThreadError(format!("Thread panicked with: {}", s))
        } else {
            Self::ThreadError("Thread panicked with an unknown type".into())
        }
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, dyn Reader + 'static>>>
    for RLanLibError
{
    fn from(value: PoisonError<MutexGuard<'a, dyn Reader + 'static>>) -> Self {
        Self::PacketReaderLock(value.to_string())
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, dyn Sender + 'static>>>
    for RLanLibError
{
    fn from(value: PoisonError<MutexGuard<'a, dyn Sender + 'static>>) -> Self {
        Self::PacketSenderLock(value.to_string())
    }
}

impl RLanLibError {
    /// Converter for std::net::AddrParseError
    pub fn from_net_addr_parse_error(
        ip: &str,
        error: std::net::AddrParseError,
    ) -> Self {
        Self::Scan {
            error: error.to_string(),
            ip: Some(ip.to_string()),
            port: None,
        }
    }

    /// Converter for ipnet::AddrParseError
    pub fn from_ipnet_addr_parse_error(
        ip: &str,
        error: ipnet::AddrParseError,
    ) -> Self {
        Self::Scan {
            error: error.to_string(),
            ip: Some(ip.to_string()),
            port: None,
        }
    }

    /// Converter for ParseIntError
    pub fn from_port_parse_int_err(port: &str, error: ParseIntError) -> Self {
        Self::Scan {
            error: error.to_string(),
            ip: None,
            port: Some(port.to_string()),
        }
    }

    /// Converter for channel send errors
    pub fn from_channel_send_error(e: SendError<ScanMessage>) -> Self {
        RLanLibError::NotifierSendError(SendError(Box::from(e.0)))
    }
}

unsafe impl Send for RLanLibError {}
unsafe impl Sync for RLanLibError {}

/// Custom Result type for this library. All Errors exposed by this library
/// will be returned as [`RLanLibError`]
pub type Result<T> = std::result::Result<T, RLanLibError>;
