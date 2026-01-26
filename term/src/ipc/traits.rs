#[cfg(test)]
use mockall::automock;

use color_eyre::eyre::Result;

#[cfg_attr(test, automock)]
pub trait IpcSender<T: Send>: Send {
    fn send(&self, m: T) -> Result<()>;
    fn box_clone(&self) -> Box<dyn IpcSender<T>>;
}

impl<T: Send> Clone for Box<dyn IpcSender<T>> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

#[cfg_attr(test, automock)]
pub trait IpcReceiver<T: Send>: Send {
    fn recv(&self) -> Result<T>;
    fn try_recv(&self) -> Result<T>;
}
