//! Local process sandbox backend.

mod exec;

use std::path::PathBuf;

use harness_contracts::ShellKind;

use crate::SandboxBaseConfig;

pub use exec::LocalActivity;

#[derive(Debug, Clone)]
pub struct LocalSandbox {
    pub(crate) base: SandboxBaseConfig,
    pub(crate) root: PathBuf,
    pub(crate) shell: ShellKind,
    pub(crate) isolation: LocalIsolation,
}

impl LocalSandbox {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_base(root, SandboxBaseConfig::default())
    }

    pub fn with_base(root: impl Into<PathBuf>, base: SandboxBaseConfig) -> Self {
        Self {
            base,
            root: root.into(),
            shell: ShellKind::System,
            isolation: LocalIsolation::None,
        }
    }

    #[must_use]
    pub fn with_shell(mut self, shell: ShellKind) -> Self {
        self.shell = shell;
        self
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn base(&self) -> &SandboxBaseConfig {
        &self.base
    }

    pub fn isolation(&self) -> LocalIsolation {
        self.isolation
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum LocalIsolation {
    None,
}
