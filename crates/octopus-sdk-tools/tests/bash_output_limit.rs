use std::process::Command;

use std::sync::Arc;

use octopus_sdk_contracts::AskAnswer;
use octopus_sdk_tools::{builtin::BashTool, Tool};
use tempfile::tempdir;

const _: [(); octopus_sdk_tools::BASH_MAX_OUTPUT_DEFAULT] = [(); 30_000];
const _: [(); octopus_sdk_tools::BASH_MAX_OUTPUT_UPPER_LIMIT] = [(); 150_000];

mod support;

#[test]
fn bash_output_default_is_30_000_chars() {
    let output = run_case("default", None);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("ok default"));
}

#[test]
fn bash_output_upper_limit_via_env() {
    let output = run_case("full", Some("150000"));
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("ok full"));
}

#[test]
fn bash_output_upper_limit_cap() {
    let output = run_case("cap", Some("200000"));
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("ok cap"));
}

fn run_case(case: &str, max_output: Option<&str>) -> std::process::Output {
    let mut command = Command::new(std::env::current_exe().expect("test binary should exist"));
    command
        .env("RUN_BASH_OUTPUT_LIMIT_CASE", case)
        .arg("--exact")
        .arg("bash_output_limit_helper")
        .arg("--nocapture");

    if let Some(value) = max_output {
        command.env("BASH_MAX_OUTPUT_LENGTH", value);
    }

    let output = command.output().expect("child test should run");
    assert!(
        output.status.success(),
        "child test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[tokio::test]
async fn bash_output_limit_helper() {
    let Some(case) = std::env::var("RUN_BASH_OUTPUT_LIMIT_CASE").ok() else {
        return;
    };

    let dir = tempdir().expect("tempdir should exist");
    let result = BashTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "ok".into(),
                        text: "ok".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({ "command": "yes x | head -c 60000" }),
        )
        .await
        .expect("bash should succeed");

    let text = support::text_output(result);
    match case.as_str() {
        "default" => {
            assert!(text.contains("[output truncated"));
            assert!(text.chars().count() < 31_000);
        }
        "full" | "cap" => {
            assert!(!text.contains("[output truncated"));
            assert_eq!(text.chars().count(), 60_000);
        }
        other => panic!("unexpected case {other}"),
    }

    println!("ok {case}");
}
