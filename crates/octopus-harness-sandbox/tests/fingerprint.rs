use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use harness_contracts::WorkspaceAccess;
use harness_sandbox::{ExecSpec, OutputOverflowPolicy, OutputPolicy, SandboxBaseConfig, StdioSpec};

#[test]
fn fingerprint_is_stable_for_env_order_and_lexical_cwd() {
    let base = SandboxBaseConfig {
        passthrough_env_keys: BTreeSet::from(["A".to_owned(), "B".to_owned()]),
        ..SandboxBaseConfig::default()
    };

    let mut first = ExecSpec {
        command: "cargo".to_owned(),
        args: vec!["test".to_owned()],
        cwd: Some(PathBuf::from("a/./b/../b")),
        ..ExecSpec::default()
    };
    first.env.insert("A".to_owned(), "1".to_owned());
    first.env.insert("B".to_owned(), "2".to_owned());
    first.env.insert("IGNORED".to_owned(), "x".to_owned());

    let mut second = ExecSpec {
        command: "cargo".to_owned(),
        args: vec!["test".to_owned()],
        cwd: Some(PathBuf::from("a/b")),
        ..ExecSpec::default()
    };
    second.env.insert("IGNORED".to_owned(), "y".to_owned());
    second.env.insert("B".to_owned(), "2".to_owned());
    second.env.insert("A".to_owned(), "1".to_owned());

    assert_eq!(
        first.canonical_fingerprint(&base),
        second.canonical_fingerprint(&base)
    );
}

#[test]
fn fingerprint_ignores_runtime_and_io_options() {
    let base = SandboxBaseConfig::default();
    let spec = ExecSpec {
        command: "echo".to_owned(),
        args: vec!["hello".to_owned()],
        ..ExecSpec::default()
    };

    let mut changed = spec.clone();
    changed.timeout = Some(Duration::from_secs(1));
    changed.activity_timeout = Some(Duration::from_secs(2));
    changed.stdin = StdioSpec::Inherit;
    changed.stdout = StdioSpec::File(PathBuf::from("out.txt"));
    changed.stderr = StdioSpec::Null;
    changed.output_policy = OutputPolicy {
        max_inline_bytes: 8,
        overflow: OutputOverflowPolicy::AbortExec,
        redact_secrets: false,
    };

    assert_eq!(
        spec.canonical_fingerprint(&base),
        changed.canonical_fingerprint(&base)
    );
}

#[test]
fn fingerprint_changes_when_workspace_access_changes() {
    let base = SandboxBaseConfig::default();
    let readonly = ExecSpec {
        command: "echo".to_owned(),
        workspace_access: WorkspaceAccess::ReadOnly,
        ..ExecSpec::default()
    };
    let readwrite = ExecSpec {
        workspace_access: WorkspaceAccess::ReadWrite {
            allowed_writable_subpaths: vec![PathBuf::from("target")],
        },
        ..readonly.clone()
    };

    assert_ne!(
        readonly.canonical_fingerprint(&base),
        readwrite.canonical_fingerprint(&base)
    );
}
