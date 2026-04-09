#![allow(clippy::must_use_candidate)]
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseCommitState {
    Matches,
    Diverged { expected: String, actual: String },
    NoExpectedBase,
    NotAGitRepo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseCommitSource {
    Flag(String),
    File(String),
}

pub fn read_claw_base_file(cwd: &Path) -> Option<String> {
    let path = cwd.join(".claw-base");
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn resolve_expected_base(flag_value: Option<&str>, cwd: &Path) -> Option<BaseCommitSource> {
    if let Some(value) = flag_value {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(BaseCommitSource::Flag(trimmed.to_string()));
        }
    }
    read_claw_base_file(cwd).map(BaseCommitSource::File)
}

pub fn check_base_commit(cwd: &Path, expected_base: Option<&BaseCommitSource>) -> BaseCommitState {
    let Some(source) = expected_base else {
        return BaseCommitState::NoExpectedBase;
    };
    let expected_raw = match source {
        BaseCommitSource::Flag(value) | BaseCommitSource::File(value) => value.as_str(),
    };

    let Some(head_sha) = resolve_rev(cwd, "HEAD") else {
        return BaseCommitState::NotAGitRepo;
    };

    let Some(expected_sha) = resolve_rev(cwd, expected_raw) else {
        return if head_sha.starts_with(expected_raw) || expected_raw.starts_with(&head_sha) {
            BaseCommitState::Matches
        } else {
            BaseCommitState::Diverged {
                expected: expected_raw.to_string(),
                actual: head_sha,
            }
        };
    };

    if head_sha == expected_sha {
        BaseCommitState::Matches
    } else {
        BaseCommitState::Diverged {
            expected: expected_sha,
            actual: head_sha,
        }
    }
}

pub fn format_stale_base_warning(state: &BaseCommitState) -> Option<String> {
    match state {
        BaseCommitState::Diverged { expected, actual } => Some(format!(
            "warning: worktree HEAD ({actual}) does not match expected base commit ({expected}). Session may run against a stale codebase."
        )),
        BaseCommitState::NotAGitRepo => {
            Some("warning: stale-base check skipped — not inside a git repository.".to_string())
        }
        BaseCommitState::Matches | BaseCommitState::NoExpectedBase => None,
    }
}

fn resolve_rev(cwd: &Path, rev: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", rev])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?;
    let trimmed = sha.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
