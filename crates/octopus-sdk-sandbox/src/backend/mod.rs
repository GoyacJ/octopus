use std::{collections::HashMap, sync::Arc};

use crate::SandboxBackend;

mod noop;
#[cfg(target_os = "linux")]
mod bubblewrap;
#[cfg(target_os = "macos")]
mod seatbelt;

#[cfg(target_os = "linux")]
pub use bubblewrap::BubblewrapBackend;
pub use noop::NoopBackend;
#[cfg(target_os = "macos")]
pub use seatbelt::SeatbeltBackend;

#[must_use]
pub fn default_backend_for_host() -> Arc<dyn SandboxBackend> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(SeatbeltBackend)
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(BubblewrapBackend::default())
    }

    #[cfg(target_os = "windows")]
    {
        tracing::warn!("sandbox fallback to Noop; Windows support TODO(W8)");
        Arc::new(NoopBackend::default())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Arc::new(NoopBackend::default())
    }
}

pub(super) fn filtered_env(allowlist: &[String]) -> HashMap<String, String> {
    std::env::vars()
        .filter(|(key, _)| allowlist.iter().any(|allowed| allowed == key))
        .collect()
}
