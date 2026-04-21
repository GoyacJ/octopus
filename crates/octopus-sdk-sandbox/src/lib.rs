//! OS-level sandbox contracts for SDK tools.

mod backend;
mod handle;
mod spec;

#[cfg(target_os = "linux")]
pub use backend::BubblewrapBackend;
#[cfg(target_os = "macos")]
pub use backend::SeatbeltBackend;
pub use backend::{default_backend_for_host, NoopBackend};
pub use handle::{SandboxHandle, SandboxHandleInner};
pub use spec::{
    NetworkProxy, SandboxBackend, SandboxCommand, SandboxError, SandboxOutput, SandboxSpec,
};
