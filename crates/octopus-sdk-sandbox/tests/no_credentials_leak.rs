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
async fn noop_backend_filters_non_allowlisted_secrets() {
    const SECRET_KEY: &str = "OCTOPUS_SDK_SANDBOX_TEST_API_KEY";
    const SECRET_VALUE: &str = "secret-xyz";

    std::env::set_var(SECRET_KEY, SECRET_VALUE);

    let root = temp_workspace("no-credentials");
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
                args: vec!["-c".into(), format!("env | grep {SECRET_KEY} || true")],
                stdin: None,
            },
        )
        .await
        .expect("noop execute should succeed");

    std::env::remove_var(SECRET_KEY);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = serde_json::to_string(&output).expect("sandbox output should serialize");

    assert!(!stdout.contains(SECRET_VALUE));
    assert!(!stdout.contains(SECRET_KEY));
    assert!(!json.contains(SECRET_VALUE));
    assert!(!json.contains(SECRET_KEY));
}
