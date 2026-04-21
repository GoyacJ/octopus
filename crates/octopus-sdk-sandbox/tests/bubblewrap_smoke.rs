#[cfg(target_os = "linux")]
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "linux")]
use octopus_sdk_sandbox::{
    BubblewrapBackend, SandboxBackend, SandboxCommand, SandboxSpec,
};

#[cfg(target_os = "linux")]
fn temp_workspace(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("octopus-sdk-sandbox-{name}-{unique}"));
    fs::create_dir_all(&root).expect("temp workspace should exist");
    root
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[cfg_attr(not(feature = "sandbox-smoke"), ignore)]
async fn bubblewrap_backend_runs_simple_command() {
    let root = temp_workspace("bubblewrap-smoke");
    let backend = BubblewrapBackend;
    let handle = backend
        .provision(SandboxSpec {
            fs_whitelist: vec![root],
            env_allowlist: vec!["PATH".into()],
            ..SandboxSpec::default()
        })
        .await
        .expect("bubblewrap provision should succeed");

    let output = backend
        .execute(
            &handle,
            SandboxCommand {
                cmd: "/bin/sh".into(),
                args: vec!["-c".into(), "echo bubblewrap".into()],
                stdin: None,
            },
        )
        .await
        .expect("bubblewrap execute should succeed");

    assert_eq!(output.exit_code, 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "bubblewrap\n");
}
