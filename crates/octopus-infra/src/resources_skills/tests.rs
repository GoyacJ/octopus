use super::*;
use serde_json::json;

fn management_capabilities() -> WorkspaceToolManagementCapabilities {
    WorkspaceToolManagementCapabilities {
        can_disable: true,
        can_edit: true,
        can_delete: true,
    }
}

fn workspace_mcp_entry(
    id: &str,
    name: &str,
    source_kind: &str,
    execution_kind: &str,
    tool_names: Vec<&str>,
    resource_uri: Option<&str>,
) -> WorkspaceToolCatalogEntry {
    serde_json::from_value(json!({
        "id": id,
        "assetId": "mcp-asset-ops",
        "capabilityId": id,
        "workspaceId": "ws-test",
        "name": name,
        "kind": "mcp",
        "sourceKind": source_kind,
        "executionKind": execution_kind,
        "description": format!("Capability for {name}"),
        "requiredPermission": null,
        "availability": "healthy",
        "sourceKey": "mcp:ops",
        "displayPath": "config/runtime/workspace.json",
        "disabled": false,
        "management": management_capabilities(),
        "builtinKey": null,
        "active": null,
        "shadowedBy": null,
        "sourceOrigin": null,
        "workspaceOwned": null,
        "relativePath": null,
        "serverName": "ops",
        "endpoint": "https://ops.example.com/mcp",
        "toolNames": tool_names,
        "resourceUri": resource_uri,
        "statusDetail": null,
        "scope": "workspace",
        "ownerScope": "workspace",
        "ownerId": "ws-test",
        "ownerLabel": "Workspace",
        "consumers": []
    }))
    .expect("mcp entry should deserialize")
}

#[test]
fn capability_management_projection_groups_mcp_capabilities_by_asset() {
    let projection = capability_management_projection(vec![
        workspace_mcp_entry(
            "mcp_tool__ops__tail_logs",
            "tail_logs",
            "mcp_tool",
            "tool",
            vec!["tail_logs"],
            None,
        ),
        workspace_mcp_entry(
            "mcp_prompt__ops__deploy_review",
            "deploy_review",
            "mcp_prompt",
            "prompt_skill",
            Vec::new(),
            None,
        ),
        workspace_mcp_entry(
            "mcp_resource__ops__guide_txt",
            "Ops Guide",
            "mcp_resource",
            "resource",
            Vec::new(),
            Some("file://ops-guide.txt"),
        ),
    ]);

    let projection_json =
        serde_json::to_value(&projection).expect("projection should serialize to JSON");

    let entries = projection_json["entries"]
        .as_array()
        .expect("entries should serialize as array");
    assert_eq!(entries.len(), 3);
    assert!(entries
        .iter()
        .any(|entry| { entry["sourceKind"] == "mcp_tool" && entry["executionKind"] == "tool" }));
    assert!(entries.iter().any(|entry| {
        entry["sourceKind"] == "mcp_prompt" && entry["executionKind"] == "prompt_skill"
    }));
    assert!(entries.iter().any(|entry| {
        entry["sourceKind"] == "mcp_resource" && entry["executionKind"] == "resource"
    }));

    let assets = projection_json["assets"]
        .as_array()
        .expect("assets should serialize as array");
    assert_eq!(
        assets.len(),
        1,
        "three MCP capabilities should collapse into one server asset"
    );
    assert_eq!(
        assets[0]["sourceKinds"],
        json!(["mcp_prompt", "mcp_resource", "mcp_tool"])
    );
    assert_eq!(
        assets[0]["executionKinds"],
        json!(["prompt_skill", "resource", "tool"])
    );

    let packages = projection_json["mcpServerPackages"]
        .as_array()
        .expect("mcp packages should serialize as array");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["toolNames"], json!(["tail_logs"]));
    assert_eq!(packages[0]["promptNames"], json!(["deploy_review"]));
    assert_eq!(packages[0]["resourceUris"], json!(["file://ops-guide.txt"]));
}
