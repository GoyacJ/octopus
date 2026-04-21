use octopus_sdk_contracts::ToolCategory;
use octopus_sdk_permissions::PermissionMode;

const EXPECTED_DECISIONS: [(PermissionMode, ToolCategory, &str); 16] = [
    (PermissionMode::Default, ToolCategory::Read, "allow"),
    (PermissionMode::Default, ToolCategory::Write, "ask"),
    (PermissionMode::Default, ToolCategory::Shell, "ask"),
    (PermissionMode::Default, ToolCategory::Subagent, "ask"),
    (PermissionMode::AcceptEdits, ToolCategory::Read, "allow"),
    (PermissionMode::AcceptEdits, ToolCategory::Write, "allow"),
    (PermissionMode::AcceptEdits, ToolCategory::Shell, "ask"),
    (PermissionMode::AcceptEdits, ToolCategory::Subagent, "allow"),
    (
        PermissionMode::BypassPermissions,
        ToolCategory::Read,
        "allow",
    ),
    (
        PermissionMode::BypassPermissions,
        ToolCategory::Write,
        "allow",
    ),
    (
        PermissionMode::BypassPermissions,
        ToolCategory::Shell,
        "allow",
    ),
    (
        PermissionMode::BypassPermissions,
        ToolCategory::Subagent,
        "allow",
    ),
    (PermissionMode::Plan, ToolCategory::Read, "allow"),
    (PermissionMode::Plan, ToolCategory::Write, "deny"),
    (PermissionMode::Plan, ToolCategory::Shell, "deny"),
    (PermissionMode::Plan, ToolCategory::Subagent, "deny"),
];

#[test]
fn permission_mode_semantics_table_is_complete() {
    assert_eq!(EXPECTED_DECISIONS.len(), 16);
}

#[test]
fn permission_mode_semantics_match_w4_decision_matrix() {
    for (mode, category, decision) in EXPECTED_DECISIONS {
        let expected = match (mode, category) {
            (PermissionMode::Default, ToolCategory::Read) => "allow",
            (PermissionMode::Default, ToolCategory::Write) => "ask",
            (PermissionMode::Default, ToolCategory::Shell) => "ask",
            (PermissionMode::Default, ToolCategory::Subagent) => "ask",
            (PermissionMode::AcceptEdits, ToolCategory::Read) => "allow",
            (PermissionMode::AcceptEdits, ToolCategory::Write) => "allow",
            (PermissionMode::AcceptEdits, ToolCategory::Shell) => "ask",
            (PermissionMode::AcceptEdits, ToolCategory::Subagent) => "allow",
            (PermissionMode::BypassPermissions, _) => "allow",
            (PermissionMode::Plan, ToolCategory::Read) => "allow",
            (PermissionMode::Plan, ToolCategory::Write) => "deny",
            (PermissionMode::Plan, ToolCategory::Shell) => "deny",
            (PermissionMode::Plan, ToolCategory::Subagent) => "deny",
            _ => unreachable!("mode/category matrix must stay aligned with EXPECTED_DECISIONS"),
        };

        assert_eq!(expected, decision, "mismatch for {mode:?} / {category:?}");
    }
}
