use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use async_trait::async_trait;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
    SandboxBackend, SandboxCommand, SandboxError, SandboxHandle, SandboxOutput, SandboxSpec,
};

use super::filtered_env;

const SEATBELT_PROFILE_TEMPLATE: &str = r"(version 1)
(deny default)
(allow process*)
(allow file-read*)
(allow file-write* {write_paths})
(allow network-outbound)
";

#[derive(Debug, Default, Clone, Copy)]
pub struct SeatbeltBackend;

#[async_trait]
impl SandboxBackend for SeatbeltBackend {
    async fn provision(&self, spec: SandboxSpec) -> Result<SandboxHandle, SandboxError> {
        let cwd = spec.fs_whitelist.first().cloned().ok_or(SandboxError::Provision {
            reason: "seatbelt backend requires fs_whitelist[0] as working directory".into(),
        })?;

        let profile = render_profile(&spec.fs_whitelist);
        let profile_path = profile_path_for(&cwd);
        fs::write(&profile_path, profile).map_err(|error| SandboxError::Provision {
            reason: error.to_string(),
        })?;

        Ok(SandboxHandle::new(cwd, spec.env_allowlist, "seatbelt"))
    }

    async fn execute(
        &self,
        handle: &SandboxHandle,
        cmd: SandboxCommand,
    ) -> Result<SandboxOutput, SandboxError> {
        let profile_path = profile_path_for(&handle.cwd().to_path_buf());
        let mut command = Command::new("sandbox-exec");
        command
            .arg("-f")
            .arg(profile_path)
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

        let output = child.wait_with_output().await.map_err(|error| SandboxError::Execute {
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

fn render_profile(paths: &[PathBuf]) -> String {
    let rendered_paths = paths
        .iter()
        .map(|path| format!("(subpath \"{}\")", path.display()))
        .collect::<Vec<_>>()
        .join(" ");

    SEATBELT_PROFILE_TEMPLATE
        .replace("{write_paths}", rendered_paths.as_str())
}

fn profile_path_for(cwd: &Path) -> PathBuf {
    cwd.join(".octopus-seatbelt.sb")
}
