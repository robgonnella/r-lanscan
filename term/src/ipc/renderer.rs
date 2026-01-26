//! IPC types for the renderer thread.

use color_eyre::eyre::{Context, Result};
use std::sync::mpsc::{Receiver, Sender};

use crate::ipc::{
    message::{MainMessage, RendererMessage},
    traits::{IpcReceiver, IpcSender},
};

/// Sends messages from the renderer to the main thread.
#[derive(Clone)]
pub struct RendererSender {
    tx: Sender<MainMessage>,
}

impl RendererSender {
    /// Creates a new sender wrapping the given channel.
    pub fn new(tx: Sender<MainMessage>) -> Self {
        Self { tx }
    }
}

impl IpcSender<MainMessage> for RendererSender {
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

/// Receives messages from the main thread in the renderer.
pub struct RendererReceiver {
    rx: Receiver<RendererMessage>,
}

impl RendererReceiver {
    /// Creates a new receiver wrapping the given channel.
    pub fn new(rx: Receiver<RendererMessage>) -> Self {
        Self { rx }
    }
}

impl IpcReceiver<RendererMessage> for RendererReceiver {
    fn recv(&self) -> Result<RendererMessage> {
        self.rx
            .recv()
            .wrap_err("failed to receive message from channel")
    }

    fn try_recv(&self) -> Result<RendererMessage> {
        self.rx
            .try_recv()
            .wrap_err("failed to receive from channel")
    }
}

/// Combined IPC handle for the renderer thread.
pub struct RendererIpc {
    pub tx: Box<dyn IpcSender<MainMessage>>,
    pub rx: Box<dyn IpcReceiver<RendererMessage>>,
}

impl RendererIpc {
    /// Creates a new IPC handle with the given sender and receiver.
    pub fn new(
        tx: Box<dyn IpcSender<MainMessage>>,
        rx: Box<dyn IpcReceiver<RendererMessage>>,
    ) -> Self {
        Self { tx, rx }
    }
}
