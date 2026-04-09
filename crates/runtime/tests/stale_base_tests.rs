use runtime::{
    check_base_commit, format_stale_base_warning, read_claw_base_file, resolve_expected_base,
    BaseCommitSource, BaseCommitState,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "runtime-stale-base-{}-{nanos}-{counter}",
        std::process::id()
    ))
}

fn run(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|error| panic!("git {} failed to execute: {error}", args.join(" ")));
    assert!(
        status.success(),
        "git {} exited with {status}",
        args.join(" ")
    );
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path).expect("create repo dir");
    run(path, &["init", "--quiet", "-b", "main"]);
    run(path, &["config", "user.email", "tests@example.com"]);
    run(path, &["config", "user.name", "Stale Base Tests"]);
    fs::write(path.join("init.txt"), "initial\n").expect("write init file");
    run(path, &["add", "."]);
    run(path, &["commit", "-m", "initial commit", "--quiet"]);
}

fn commit_file(repo: &Path, name: &str, msg: &str) {
    fs::write(repo.join(name), format!("{msg}\n")).expect("write file");
    run(repo, &["add", name]);
    run(repo, &["commit", "-m", msg, "--quiet"]);
}

fn head_sha(repo: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .expect("git rev-parse HEAD");
    String::from_utf8(output.stdout)
        .expect("valid utf8")
        .trim()
        .to_string()
}

#[test]
fn stale_base_matches_when_head_equals_expected_base() {
    let root = temp_dir();
    init_repo(&root);
    let sha = head_sha(&root);
    let source = BaseCommitSource::Flag(sha);

    let state = check_base_commit(&root, Some(&source));

    assert_eq!(state, BaseCommitState::Matches);
    fs::remove_dir_all(&root).expect("cleanup");
}

#[test]
fn stale_base_diverged_warning_uses_expected_and_actual_commits() {
    let root = temp_dir();
    init_repo(&root);
    let old_sha = head_sha(&root);
    commit_file(&root, "extra.txt", "move head forward");
    let new_sha = head_sha(&root);
    let source = BaseCommitSource::Flag(old_sha.clone());

    let state = check_base_commit(&root, Some(&source));
    let warning = format_stale_base_warning(&state).expect("warning should render");

    assert_eq!(
        state,
        BaseCommitState::Diverged {
            expected: old_sha.clone(),
            actual: new_sha.clone(),
        }
    );
    assert!(warning.contains(&old_sha));
    assert!(warning.contains(&new_sha));
    fs::remove_dir_all(&root).expect("cleanup");
}

#[test]
fn stale_base_resolves_expected_commit_from_flag_or_file() {
    let root = temp_dir();
    fs::create_dir_all(&root).expect("create dir");
    fs::write(root.join(".claw-base"), "abc1234def5678\n").expect("write .claw-base");

    assert_eq!(
        read_claw_base_file(&root),
        Some("abc1234def5678".to_string())
    );
    assert_eq!(
        resolve_expected_base(Some("  flag123  "), &root),
        Some(BaseCommitSource::Flag("flag123".to_string()))
    );
    assert_eq!(
        resolve_expected_base(None, &root),
        Some(BaseCommitSource::File("abc1234def5678".to_string()))
    );

    fs::remove_dir_all(&root).expect("cleanup");
}
