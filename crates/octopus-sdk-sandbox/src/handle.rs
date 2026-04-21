use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

pub trait SandboxHandleInner: Send + Sync {
    fn cwd(&self) -> &Path;
    fn env_allowlist(&self) -> &[String];
    fn backend_name(&self) -> &'static str;
}

#[derive(Clone)]
pub struct SandboxHandle {
    inner: Arc<dyn SandboxHandleInner>,
}

impl SandboxHandle {
    #[must_use]
    pub fn from_inner(inner: Arc<dyn SandboxHandleInner>) -> Self {
        Self { inner }
    }

    #[must_use]
    pub fn new(cwd: PathBuf, env_allowlist: Vec<String>, backend_name: &'static str) -> Self {
        Self::from_inner(Arc::new(StaticSandboxHandle {
            cwd,
            env_allowlist,
            backend_name,
        }))
    }

    #[must_use]
    pub fn cwd(&self) -> &Path {
        self.inner.cwd()
    }

    #[must_use]
    pub fn env_allowlist(&self) -> &[String] {
        self.inner.env_allowlist()
    }

    #[must_use]
    pub fn backend_name(&self) -> &'static str {
        self.inner.backend_name()
    }
}

impl fmt::Debug for SandboxHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SandboxHandle")
            .field("cwd", &self.cwd())
            .field("env_allowlist", &self.env_allowlist())
            .field("backend_name", &self.backend_name())
            .finish()
    }
}

struct StaticSandboxHandle {
    cwd: PathBuf,
    env_allowlist: Vec<String>,
    backend_name: &'static str,
}

impl SandboxHandleInner for StaticSandboxHandle {
    fn cwd(&self) -> &Path {
        &self.cwd
    }

    fn env_allowlist(&self) -> &[String] {
        &self.env_allowlist
    }

    fn backend_name(&self) -> &'static str {
        self.backend_name
    }
}
