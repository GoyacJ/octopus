use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinToolPermission {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl BuiltinToolPermission {
    #[must_use]
    pub fn as_catalog_value(self) -> &'static str {
        match self {
            Self::ReadOnly => "readonly",
            Self::WorkspaceWrite => "workspace-write",
            Self::DangerFullAccess => "danger-full-access",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinToolMetadata {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub required_permission: BuiltinToolPermission,
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinToolCatalog {
    entries: &'static [BuiltinToolMetadata],
}

impl BuiltinToolCatalog {
    #[must_use]
    pub fn entries(self) -> &'static [BuiltinToolMetadata] {
        self.entries
    }

    #[must_use]
    pub fn names(self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.name.to_string())
            .collect()
    }

    #[must_use]
    pub fn name_set(self) -> BTreeSet<String> {
        self.entries
            .iter()
            .map(|entry| entry.name.to_string())
            .collect()
    }

    #[must_use]
    pub fn resolve(self, name: &str) -> Option<&'static BuiltinToolMetadata> {
        self.entries.iter().find(|entry| {
            entry.name.eq_ignore_ascii_case(name)
                || entry
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(name))
        })
    }
}

#[must_use]
pub fn builtin_tool_catalog() -> BuiltinToolCatalog {
    BuiltinToolCatalog {
        entries: BUILTIN_TOOL_ENTRIES,
    }
}

const BUILTIN_TOOL_ENTRIES: &[BuiltinToolMetadata] = &[
    BuiltinToolMetadata {
        name: "read_file",
        aliases: &["read"],
        description: "Read a text file from the workspace.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "write_file",
        aliases: &["write"],
        description: "Write a text file in the workspace.",
        required_permission: BuiltinToolPermission::WorkspaceWrite,
    },
    BuiltinToolMetadata {
        name: "edit_file",
        aliases: &["edit"],
        description: "Replace text in a workspace file.",
        required_permission: BuiltinToolPermission::WorkspaceWrite,
    },
    BuiltinToolMetadata {
        name: "glob",
        aliases: &["glob_search"],
        description: "Expand a glob against the current workspace.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "grep",
        aliases: &["grep_search", "rg"],
        description: "Run a regex search across workspace files.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "bash",
        aliases: &[],
        description: "Execute a shell command in the current workspace.",
        required_permission: BuiltinToolPermission::DangerFullAccess,
    },
    BuiltinToolMetadata {
        name: "web_search",
        aliases: &[],
        description: "Query the web when external verification is required.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "web_fetch",
        aliases: &[],
        description: "Fetch a web page when a direct source is required.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "ask_user_question",
        aliases: &["AskUserQuestion"],
        description: "Ask the user a question and wait for their response.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "todo_write",
        aliases: &["TodoWrite"],
        description: "Update the structured task list for the current session.",
        required_permission: BuiltinToolPermission::WorkspaceWrite,
    },
    BuiltinToolMetadata {
        name: "sleep",
        aliases: &["Sleep"],
        description: "Wait for a specified duration without holding a shell process.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "task",
        aliases: &["agent"],
        description: "Spawn and manage subagent execution.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "skill",
        aliases: &["SkillTool"],
        description: "Resolve and activate a named skill package.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "task_list",
        aliases: &["TaskList"],
        description: "List background tasks tracked by the host runtime.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
    BuiltinToolMetadata {
        name: "task_get",
        aliases: &["TaskGet"],
        description: "Read a single background task snapshot from the host runtime.",
        required_permission: BuiltinToolPermission::ReadOnly,
    },
];

#[cfg(test)]
mod tests {
    use super::{builtin_tool_catalog, BuiltinToolPermission};

    #[test]
    fn builtin_tool_catalog_resolves_aliases_to_canonical_names() {
        let catalog = builtin_tool_catalog();

        assert_eq!(catalog.resolve("read").map(|entry| entry.name), Some("read_file"));
        assert_eq!(catalog.resolve("rg").map(|entry| entry.name), Some("grep"));
        assert_eq!(
            catalog.resolve("AskUserQuestion").map(|entry| entry.name),
            Some("ask_user_question")
        );
    }

    #[test]
    fn builtin_tool_permissions_keep_catalog_strings() {
        assert_eq!(BuiltinToolPermission::ReadOnly.as_catalog_value(), "readonly");
        assert_eq!(
            BuiltinToolPermission::WorkspaceWrite.as_catalog_value(),
            "workspace-write"
        );
        assert_eq!(
            BuiltinToolPermission::DangerFullAccess.as_catalog_value(),
            "danger-full-access"
        );
    }
}
