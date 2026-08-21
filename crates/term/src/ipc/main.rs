//! IPC types for the main event handler thread.

use color_eyre::eyre::{Context, Result};
use std::sync::mpsc::{Receiver, Sender};

use crate::ipc::{
    message::{MainMessage, NetworkMessage, RendererMessage},
    traits::{IpcReceiver, IpcSender},
};

/// Sends messages from the main thread to the renderer.
#[derive(Clone)]
pub struct MainRendererSender {
    renderer_tx: Sender<RendererMessage>,
}

impl MainRendererSender {
    /// Creates a new sender wrapping the given channel.
    pub fn new(renderer_tx: Sender<RendererMessage>) -> Self {
        Self { renderer_tx }
    }
}

impl IpcSender<RendererMessage> for MainRendererSender {
    fn send(&self, m: RendererMessage) -> Result<()> {
        self.renderer_tx.send(m)?;
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn IpcSender<RendererMessage>> {
        Box::new(Self {
            renderer_tx: self.renderer_tx.clone(),
        })
    }
}

/// Sends messages from the main thread to the network thread.
#[derive(Clone)]
pub struct MainNetworkSender {
    network_tx: Sender<NetworkMessage>,
}

impl MainNetworkSender {
    /// Creates a new sender wrapping the given channel.
    pub fn new(network_tx: Sender<NetworkMessage>) -> Self {
        Self { network_tx }
    }
}

impl IpcSender<NetworkMessage> for MainNetworkSender {
    fn send(&self, m: NetworkMessage) -> Result<()> {
        self.network_tx.send(m)?;
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn IpcSender<NetworkMessage>> {
        Box::new(Self {
            network_tx: self.network_tx.clone(),
        })
    }
}

/// Receives messages from the renderer in the main thread.
pub struct MainReceiver {
    rx: Receiver<MainMessage>,
}

impl MainReceiver {
    /// Creates a new receiver wrapping the given channel.
    pub fn new(rx: Receiver<MainMessage>) -> Self {
        Self { rx }
    }
}

impl IpcReceiver<MainMessage> for MainReceiver {
    fn recv(&self) -> Result<MainMessage> {
        self.rx
            .recv()
            .wrap_err("failed to receive message from channel")
    }

    fn try_recv(&self) -> Result<MainMessage> {
        self.rx
            .try_recv()
            .wrap_err("failed to receive from channel")
    }
}

/// Combined IPC handle for the main event handler thread.
pub struct MainIpc {
    pub renderer_tx: Box<dyn IpcSender<RendererMessage>>,
    pub network_tx: Box<dyn IpcSender<NetworkMessage>>,
    pub rx: Box<dyn IpcReceiver<MainMessage>>,
    /// Sender side of the main channel, used to post messages back to
    /// the main event loop from spawned threads (e.g. traceroute).
    pub tx: Sender<MainMessage>,
}

impl MainIpc {
    /// Creates a new IPC handle with the given sender and receiver.
    pub fn new(
        renderer_tx: Box<dyn IpcSender<RendererMessage>>,
        network_tx: Box<dyn IpcSender<NetworkMessage>>,
        rx: Box<dyn IpcReceiver<MainMessage>>,
        tx: Sender<MainMessage>,
    ) -> Self {
        Self {
            renderer_tx,
            network_tx,
            rx,
            tx,
        }
    }
}
