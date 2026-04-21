use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::SessionId;

fn temp_workspace(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("octopus-sdk-context-{name}-{unique}"));
    fs::create_dir_all(&root).expect("temp workspace should exist");
    root
}

#[tokio::test]
async fn concurrent_writes_keep_last_completed_snapshot() {
    let root = temp_workspace("scratchpad");
    let scratchpad = DurableScratchpad::new(root);
    let session = SessionId("session-scratchpad".into());

    let mut handles = Vec::new();
    for index in 0..10u64 {
        let scratchpad = scratchpad.clone();
        let session = session.clone();
        handles.push(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(index * 10)).await;
            let content = format!("snapshot-{index}");
            scratchpad
                .write(&session, &content)
                .await
                .expect("write should succeed");
        }));
    }

    for handle in handles {
        handle.await.expect("join should succeed");
    }

    let content = scratchpad
        .read(&session)
        .await
        .expect("read should succeed")
        .expect("content should exist");

    assert_eq!(content, "snapshot-9");
}
