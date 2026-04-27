#![cfg(feature = "builtin-toolset")]

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream, StreamExt};
use harness_contracts::{
    BlobError, BlobMeta, BlobRef, BlobRetention, BlobStore, CapabilityRegistry, Decision,
    DecisionScope, PermissionError, PermissionSubject, TenantId, ToolError, ToolResult, ToolUseId,
};
use harness_permission::{PermissionBroker, PermissionCheck, PermissionContext, PermissionRequest};
use harness_tool::{
    builtin::{FileReadTool, FileWriteTool, GrepTool, ListDirTool, ReadBlobTool},
    BuiltinToolset, InterruptToken, Tool, ToolContext, ToolRegistry,
};
use serde_json::{json, Value};
use tempfile::tempdir;

#[tokio::test]
async fn file_read_reads_utf8_and_line_ranges() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
    let tool = FileReadTool::default();

    assert_asks_for_permission(&tool, json!({ "path": file })).await;

    let result = execute_final(
        &tool,
        json!({
            "path": file,
            "start_line": 2,
            "end_line": 3
        }),
        tool_ctx(CapabilityRegistry::default()),
    )
    .await;

    assert_eq!(result, ToolResult::Text("two\nthree\n".to_owned()));
}

#[tokio::test]
async fn file_write_overwrites_file_and_asks_for_path_permission() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("out.txt");
    let tool = FileWriteTool::default();

    let check = tool
        .check_permission(
            &json!({ "path": file, "content": "new" }),
            &tool_ctx(CapabilityRegistry::default()),
        )
        .await;
    assert!(matches!(
        check,
        PermissionCheck::AskUser {
            subject: PermissionSubject::FileWrite { .. },
            scope: DecisionScope::PathPrefix(_)
        }
    ));

    let result = execute_final(
        &tool,
        json!({ "path": file, "content": "new" }),
        tool_ctx(CapabilityRegistry::default()),
    )
    .await;

    assert_eq!(std::fs::read_to_string(&file).unwrap(), "new");
    assert_eq!(
        result,
        ToolResult::Structured(json!({ "path": file, "bytes": 3 }))
    );
}

#[tokio::test]
async fn list_dir_is_stable_and_hides_dotfiles_by_default() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("b.txt"), "b").unwrap();
    std::fs::write(dir.path().join(".hidden"), "hidden").unwrap();
    std::fs::create_dir(dir.path().join("a_dir")).unwrap();
    let tool = ListDirTool::default();

    let result = execute_final(
        &tool,
        json!({ "path": dir.path() }),
        tool_ctx(CapabilityRegistry::default()),
    )
    .await;

    let ToolResult::Structured(value) = result else {
        panic!("expected structured list result");
    };
    let names: Vec<_> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["a_dir", "b.txt"]);
}

#[tokio::test]
async fn grep_uses_rg_and_returns_matches() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "alpha\nneedle\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "other\nneedle two\n").unwrap();
    let tool = GrepTool::default();

    let result = execute_final(
        &tool,
        json!({ "path": dir.path(), "pattern": "needle" }),
        tool_ctx(CapabilityRegistry::default()),
    )
    .await;

    let ToolResult::Structured(value) = result else {
        panic!("expected structured grep result");
    };
    let matches = value.as_array().unwrap();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0]["line"], 2);
    assert!(matches[0]["path"].as_str().unwrap().ends_with("a.txt"));
    assert_eq!(matches[1]["line"], 2);
    assert!(matches[1]["path"].as_str().unwrap().ends_with("b.txt"));
}

#[tokio::test]
async fn read_blob_reads_from_capability_registry_and_reports_missing_store() {
    let blob_ref = BlobRef {
        id: harness_contracts::BlobId::new(),
        size: 5,
        content_hash: [9; 32],
        content_type: Some("text/plain".to_owned()),
    };
    let store: Arc<dyn BlobStore> = Arc::new(TestBlobStore {
        blob_ref: blob_ref.clone(),
        bytes: Bytes::from_static(b"hello"),
    });
    let mut caps = CapabilityRegistry::default();
    caps.install(harness_contracts::ToolCapability::BlobReader, store);
    let tool = ReadBlobTool::default();

    let result = execute_final(&tool, json!({ "blob_ref": blob_ref }), tool_ctx(caps)).await;
    assert_eq!(result, ToolResult::Text("hello".to_owned()));

    let error = execute_error(
        &tool,
        json!({ "blob_ref": blob_ref }),
        tool_ctx(CapabilityRegistry::default()),
    )
    .await;
    assert!(matches!(
        error,
        ToolError::CapabilityMissing(harness_contracts::ToolCapability::BlobReader)
    ));
}

#[test]
fn default_builtin_toolset_registers_m3_t04a_tools_without_model_or_journal_deps() {
    let registry = ToolRegistry::builder()
        .with_builtin_toolset(BuiltinToolset::Default)
        .build()
        .unwrap();

    for name in ["FileRead", "FileWrite", "ListDir", "Grep", "ReadBlob"] {
        assert!(registry.get(name).is_some(), "{name} should be registered");
    }

    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    assert!(!manifest.contains("octopus-harness-model"));
    assert!(!manifest.contains("octopus-harness-journal"));
}

async fn assert_asks_for_permission(tool: &dyn Tool, input: Value) {
    let check = tool
        .check_permission(&input, &tool_ctx(CapabilityRegistry::default()))
        .await;
    assert!(matches!(check, PermissionCheck::AskUser { .. }));
}

async fn execute_final(tool: &dyn Tool, input: Value, ctx: ToolContext) -> ToolResult {
    tool.validate(&input, &ctx).await.unwrap();
    let mut stream = tool.execute(input, ctx).await.unwrap();
    match stream.next().await {
        Some(harness_tool::ToolEvent::Final(result)) => result,
        other => panic!("expected final result, got {other:?}"),
    }
}

async fn execute_error(tool: &dyn Tool, input: Value, ctx: ToolContext) -> ToolError {
    tool.validate(&input, &ctx).await.unwrap();
    match tool.execute(input, ctx).await {
        Ok(_) => panic!("expected tool error"),
        Err(error) => error,
    }
}

fn tool_ctx(cap_registry: CapabilityRegistry) -> ToolContext {
    ToolContext {
        tool_use_id: ToolUseId::new(),
        run_id: harness_contracts::RunId::new(),
        session_id: harness_contracts::SessionId::new(),
        tenant_id: TenantId::SINGLE,
        workspace_root: std::env::temp_dir(),
        sandbox: None,
        permission_broker: Arc::new(AllowBroker),
        cap_registry: Arc::new(cap_registry),
        interrupt: InterruptToken::default(),
        parent_run: None,
    }
}

#[derive(Debug)]
struct AllowBroker;

#[async_trait]
impl PermissionBroker for AllowBroker {
    async fn decide(&self, _request: PermissionRequest, _ctx: PermissionContext) -> Decision {
        Decision::AllowOnce
    }

    async fn persist(
        &self,
        _decision_id: harness_contracts::DecisionId,
        _scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        Ok(())
    }
}

#[derive(Clone)]
struct TestBlobStore {
    blob_ref: BlobRef,
    bytes: Bytes,
}

#[async_trait]
impl BlobStore for TestBlobStore {
    fn store_id(&self) -> &'static str {
        "test"
    }

    async fn put(
        &self,
        _tenant: TenantId,
        _bytes: Bytes,
        _meta: BlobMeta,
    ) -> Result<BlobRef, BlobError> {
        Ok(self.blob_ref.clone())
    }

    async fn get(
        &self,
        _tenant: TenantId,
        blob: &BlobRef,
    ) -> Result<futures::stream::BoxStream<'static, Bytes>, BlobError> {
        if blob.id != self.blob_ref.id {
            return Err(BlobError::NotFound(blob.id));
        }
        Ok(Box::pin(stream::once({
            let bytes = self.bytes.clone();
            async move { bytes }
        })))
    }

    async fn head(&self, _tenant: TenantId, blob: &BlobRef) -> Result<Option<BlobMeta>, BlobError> {
        Ok((blob.id == self.blob_ref.id).then(|| BlobMeta {
            content_type: self.blob_ref.content_type.clone(),
            size: self.blob_ref.size,
            content_hash: self.blob_ref.content_hash,
            created_at: chrono::Utc::now(),
            retention: BlobRetention::TenantScoped,
        }))
    }

    async fn delete(&self, _tenant: TenantId, _blob: &BlobRef) -> Result<(), BlobError> {
        Ok(())
    }
}
