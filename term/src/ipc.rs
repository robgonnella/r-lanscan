//! Inter-process communication between the renderer and main event handler.
//!
//! Provides thread-safe message passing for UI lifecycle events (pause/resume)
//! and command execution requests.

pub mod main;
pub mod message;
pub mod network;
pub mod renderer;
pub mod traits;
