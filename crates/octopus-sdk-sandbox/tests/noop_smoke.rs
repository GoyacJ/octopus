#![cfg(unix)]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use octopus_sdk_sandbox::{NoopBackend, SandboxBackend, SandboxCommand, SandboxSpec};

fn temp_workspace(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("octopus-sdk-sandbox-{name}-{unique}"));
    fs::create_dir_all(&root).expect("temp workspace should exist");
    root
}

#[tokio::test]
async fn noop_backend_runs_simple_command() {
    let root = temp_workspace("noop-smoke");
    let backend = NoopBackend;
    let handle = backend
        .provision(SandboxSpec {
            fs_whitelist: vec![root],
            env_allowlist: vec!["PATH".into()],
            ..SandboxSpec::default()
        })
        .await
        .expect("noop provision should succeed");

    let output = backend
        .execute(
            &handle,
            SandboxCommand {
                cmd: "/bin/sh".into(),
                args: vec!["-c".into(), "echo hello".into()],
                stdin: None,
            },
        )
        .await
        .expect("noop execute should succeed");

    assert_eq!(output.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}
