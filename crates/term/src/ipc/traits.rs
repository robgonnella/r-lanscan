//! Traits for IPC sender and receiver abstractions.

#[cfg(test)]
use mockall::automock;

use color_eyre::eyre::Result;

/// Sends messages of type T between threads. Cloneable for sharing.
#[cfg_attr(test, automock)]
pub trait IpcSender<T: Send>: Send {
    /// Sends a message, blocking until the receiver is ready.
    fn send(&self, m: T) -> Result<()>;
    /// Creates a boxed clone for sharing across threads.
    fn box_clone(&self) -> Box<dyn IpcSender<T>>;
}

impl<T: Send> Clone for Box<dyn IpcSender<T>> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Receives messages of type T from another thread.
#[cfg_attr(test, automock)]
pub trait IpcReceiver<T: Send>: Send {
    /// Blocks until a message is received.
    fn recv(&self) -> Result<T>;
    /// Returns immediately with a message or error if none available.
    fn try_recv(&self) -> Result<T>;
}
