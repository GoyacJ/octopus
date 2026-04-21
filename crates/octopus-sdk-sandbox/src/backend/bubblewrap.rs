use std::process::Stdio;

use async_trait::async_trait;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
    SandboxBackend, SandboxCommand, SandboxError, SandboxHandle, SandboxOutput, SandboxSpec,
};

use super::filtered_env;

#[derive(Debug, Default, Clone, Copy)]
pub struct BubblewrapBackend;

#[async_trait]
impl SandboxBackend for BubblewrapBackend {
    async fn provision(&self, spec: SandboxSpec) -> Result<SandboxHandle, SandboxError> {
        let cwd = spec
            .fs_whitelist
            .first()
            .cloned()
            .ok_or(SandboxError::Provision {
                reason: "bubblewrap backend requires fs_whitelist[0] as working directory".into(),
            })?;

        Ok(SandboxHandle::new(cwd, spec.env_allowlist, "bubblewrap"))
    }

    async fn execute(
        &self,
        handle: &SandboxHandle,
        cmd: SandboxCommand,
    ) -> Result<SandboxOutput, SandboxError> {
        let mut command = Command::new("bwrap");
        command
            .arg("--die-with-parent")
            .arg("--new-session")
            .arg("--unshare-all")
            .arg("--ro-bind")
            .arg("/")
            .arg("/")
            .arg("--bind")
            .arg(handle.cwd())
            .arg(handle.cwd())
            .arg("--chdir")
            .arg(handle.cwd())
            .arg("--")
            .arg(&cmd.cmd)
            .args(&cmd.args)
            .current_dir(handle.cwd())
            .stdin(if cmd.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .envs(filtered_env(handle.env_allowlist()));

        let mut child = command.spawn().map_err(|error| SandboxError::Execute {
            reason: error.to_string(),
        })?;

        if let Some(stdin) = cmd.stdin {
            let mut child_stdin = child.stdin.take().ok_or(SandboxError::Execute {
                reason: "stdin pipe was not available".into(),
            })?;
            child_stdin
                .write_all(&stdin)
                .await
                .map_err(|error| SandboxError::Execute {
                    reason: error.to_string(),
                })?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|error| SandboxError::Execute {
                reason: error.to_string(),
            })?;

        Ok(SandboxOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: false,
            timed_out: false,
        })
    }

    async fn terminate(&self, _handle: SandboxHandle) -> Result<(), SandboxError> {
        Ok(())
    }
}
