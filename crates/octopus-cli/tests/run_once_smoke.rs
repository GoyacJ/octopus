use std::sync::{Mutex, OnceLock};

use octopus_cli::run_once::main_with_args;

fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn run_once_smoke_lists_current_directory_through_harness_m3_driver() {
    let _guard = cwd_lock().lock().expect("cwd lock should remain available");
    let workspace = tempfile::tempdir().expect("workspace should exist");
    std::fs::write(workspace.path().join("m3-run-once-marker.txt"), "m3")
        .expect("marker should be written");
    let previous_cwd = std::env::current_dir().expect("current dir should resolve");
    std::env::set_current_dir(workspace.path()).expect("cwd should switch");

    let mut out = Vec::new();
    let result = main_with_args(
        vec![
            "octopus-cli".to_owned(),
            "run".to_owned(),
            "--once".to_owned(),
            "list current dir".to_owned(),
        ],
        &mut out,
    )
    .await;

    std::env::set_current_dir(previous_cwd).expect("cwd should restore");
    result.expect("run --once should succeed");

    let rendered = String::from_utf8(out).expect("stdout should stay utf8");
    assert!(rendered.contains("[tool.executed] name=ListDir"));
    assert!(rendered.contains("m3-run-once-marker.txt"));
}
