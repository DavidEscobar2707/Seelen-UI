pub mod app;
pub mod commands;
pub mod common;
pub mod error;
pub mod messages;
pub mod security;

// Re-export main types for convenience
pub use app::AppIpc;
pub use common::IPC;
