use std::{fs, path::Path};

use octopus_sdk_contracts::SubagentError;
use octopus_sdk_subagent::AgentRegistry;
use tempfile::tempdir;

const REVIEWER_MD: &str = include_str!("fixtures/agents/reviewer.md");

#[test]
fn test_parse_reviewer_md() {
    let root = tempdir().expect("tempdir should exist");
    write_agent(root.path(), "reviewer.md", REVIEWER_MD);

    let registry =
        AgentRegistry::discover(&[root.path().to_path_buf()]).expect("registry should parse");
    let spec = registry
        .get("reviewer")
        .expect("reviewer agent should exist");

    assert_eq!(spec.id, "reviewer");
    assert_eq!(spec.model_role, "main");
    assert_eq!(
        spec.allowed_tools,
        vec!["fs_read", "fs_grep", "fs_glob", "ask_user_question"]
    );
    assert_eq!(spec.max_turns, 20);
    assert_eq!(spec.task_budget.total, 40_000);
    assert_eq!(spec.task_budget.completion_threshold, 0.9);
    assert_eq!(spec.depth, 1);
    assert!(spec.system_prompt.contains("Review Checklist"));
}

#[test]
fn test_workspace_shadow_project() {
    let project = tempdir().expect("project tempdir should exist");
    let workspace = tempdir().expect("workspace tempdir should exist");
    write_agent(project.path(), "reviewer.md", REVIEWER_MD);
    write_agent(
        workspace.path(),
        "reviewer.md",
        &REVIEWER_MD.replace("task_budget: 40000", "task_budget: 10000"),
    );

    let registry =
        AgentRegistry::discover(&[project.path().to_path_buf(), workspace.path().to_path_buf()])
            .expect("registry should parse");
    let spec = registry
        .get("reviewer")
        .expect("workspace agent should shadow project agent");

    assert_eq!(spec.task_budget.total, 10_000);
}

#[test]
fn test_invalid_id_rejected() {
    let root = tempdir().expect("tempdir should exist");
    write_agent(
        root.path(),
        "invalid.md",
        r#"---
name: Reviewer!
model: claude-sonnet-4-5
---

invalid
"#,
    );

    let error =
        AgentRegistry::discover(&[root.path().to_path_buf()]).expect_err("invalid id should fail");

    assert_eq!(
        error,
        SubagentError::Storage {
            reason: "invalid agent id".into(),
        }
    );
}

fn write_agent(root: &Path, filename: &str, content: &str) {
    let agents_dir = root.join(".agents");
    fs::create_dir_all(&agents_dir).expect("agents dir should exist");
    fs::write(agents_dir.join(filename), content).expect("agent file should write");
}
