use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use octopus_sdk_sandbox::{
    default_backend_for_host, SandboxHandle, SandboxHandleInner, SandboxSpec,
};

struct FixtureHandle {
    cwd: PathBuf,
    env_allowlist: Vec<String>,
}

impl SandboxHandleInner for FixtureHandle {
    fn cwd(&self) -> &Path {
        &self.cwd
    }

    fn env_allowlist(&self) -> &[String] {
        &self.env_allowlist
    }

    fn backend_name(&self) -> &'static str {
        "fixture"
    }
}

#[test]
fn sandbox_handle_accessors_expose_inner_values() {
    let handle = SandboxHandle::from_inner(Arc::new(FixtureHandle {
        cwd: PathBuf::from("/tmp/workspace"),
        env_allowlist: vec!["PATH".into(), "HOME".into()],
    }));

    assert_eq!(handle.cwd(), Path::new("/tmp/workspace"));
    assert_eq!(handle.env_allowlist(), ["PATH", "HOME"]);
    assert_eq!(handle.backend_name(), "fixture");
}

#[test]
fn sandbox_spec_keeps_w4_limits_shape() {
    let spec = SandboxSpec {
        fs_whitelist: vec![PathBuf::from("/tmp/workspace")],
        env_allowlist: vec!["PATH".into()],
        cpu_time_limit_ms: Some(100),
        wall_time_limit_ms: Some(200),
        memory_limit_bytes: Some(4096),
        ..SandboxSpec::default()
    };

    assert_eq!(spec.fs_whitelist, [PathBuf::from("/tmp/workspace")]);
    assert_eq!(spec.env_allowlist, ["PATH"]);
    assert_eq!(spec.cpu_time_limit_ms, Some(100));
    assert_eq!(spec.wall_time_limit_ms, Some(200));
    assert_eq!(spec.memory_limit_bytes, Some(4096));
}

#[tokio::test]
async fn default_backend_for_host_provisions_a_handle() {
    let root = std::env::temp_dir().join("octopus-sdk-sandbox-default-backend");
    std::fs::create_dir_all(&root).expect("temp workspace should exist");
    let backend = default_backend_for_host();

    let handle = backend
        .provision(SandboxSpec {
            fs_whitelist: vec![root],
            env_allowlist: vec!["PATH".into()],
            ..SandboxSpec::default()
        })
        .await
        .expect("default backend should provision");

    assert!(!handle.backend_name().is_empty());
}
