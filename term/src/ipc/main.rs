use color_eyre::eyre::{Context, Result};
use std::sync::mpsc::{Receiver, Sender};

use crate::ipc::{
    message::{MainMessage, RendererMessage},
    traits::{IpcReceiver, IpcSender},
};

#[derive(Clone)]
pub struct MainSender {
    tx: Sender<RendererMessage>,
}

impl MainSender {
    pub fn new(tx: Sender<RendererMessage>) -> Self {
        Self { tx }
    }
}

impl IpcSender<RendererMessage> for MainSender {
    fn send(&self, m: RendererMessage) -> Result<()> {
        self.tx.send(m)?;
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn IpcSender<RendererMessage>> {
        Box::new(Self {
            tx: self.tx.clone(),
        })
    }
}

pub struct MainReceiver {
    rx: Receiver<MainMessage>,
}

impl MainReceiver {
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

pub struct MainIpc {
    pub tx: Box<dyn IpcSender<RendererMessage>>,
    pub rx: Box<dyn IpcReceiver<MainMessage>>,
}

impl MainIpc {
    pub fn new(
        tx: Box<dyn IpcSender<RendererMessage>>,
        rx: Box<dyn IpcReceiver<MainMessage>>,
    ) -> Self {
        Self { tx, rx }
    }
}
