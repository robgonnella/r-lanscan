//! IPC types for the main event handler thread.

use color_eyre::eyre::{Context, Result};
use std::sync::mpsc::{Receiver, Sender};

use crate::ipc::{
    message::{MainMessage, NetworkMessage},
    traits::{IpcReceiver, IpcSender},
};

/// Sends messages from the main thread to the renderer.
#[derive(Clone)]
pub struct NetworkSender {
    tx: Sender<MainMessage>,
}

impl NetworkSender {
    /// Creates a new sender wrapping the given channel.
    pub fn new(tx: Sender<MainMessage>) -> Self {
        Self { tx }
    }
}

impl IpcSender<MainMessage> for NetworkSender {
    fn send(&self, m: MainMessage) -> Result<()> {
        self.tx.send(m)?;
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn IpcSender<MainMessage>> {
        Box::new(Self {
            tx: self.tx.clone(),
        })
    }
}

/// Receives messages from the renderer in the main thread.
pub struct NetworkReceiver {
    rx: Receiver<NetworkMessage>,
}

impl NetworkReceiver {
    /// Creates a new receiver wrapping the given channel.
    pub fn new(rx: Receiver<NetworkMessage>) -> Self {
        Self { rx }
    }
}

impl IpcReceiver<NetworkMessage> for NetworkReceiver {
    fn recv(&self) -> Result<NetworkMessage> {
        self.rx
            .recv()
            .wrap_err("failed to receive message from channel")
    }

    fn try_recv(&self) -> Result<NetworkMessage> {
        self.rx
            .try_recv()
            .wrap_err("failed to receive from channel")
    }
}

/// Combined IPC handle for the main event handler thread.
pub struct NetworkIpc {
    pub tx: Box<dyn IpcSender<MainMessage>>,
    pub rx: Box<dyn IpcReceiver<NetworkMessage>>,
}

impl NetworkIpc {
    /// Creates a new IPC handle with the given sender and receiver.
    pub fn new(
        tx: Box<dyn IpcSender<MainMessage>>,
        rx: Box<dyn IpcReceiver<NetworkMessage>>,
    ) -> Self {
        Self { tx, rx }
    }
}
