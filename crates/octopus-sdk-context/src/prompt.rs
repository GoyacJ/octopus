use std::path::PathBuf;

use octopus_sdk_contracts::{PermissionMode, SessionId};
use octopus_sdk_tools::ToolSurface;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptSection {
    pub id: &'static str,
    pub order: u32,
    pub body: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemPromptBuilder {
    sections: Vec<SystemPromptSection>,
}

#[derive(Clone)]
pub struct PromptCtx<'a> {
    pub session: SessionId,
    pub mode: PermissionMode,
    pub project_root: PathBuf,
    pub tools: &'a ToolSurface,
}

impl SystemPromptBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_section(mut self, section: SystemPromptSection) -> Self {
        self.sections.push(section);
        self
    }

    #[must_use]
    pub fn build(&self, ctx: &PromptCtx<'_>) -> Vec<String> {
        let mut sections = vec![
            role_section(ctx),
            tools_guidance_section(ctx.tools),
            process_section(),
            safety_section(),
            output_section(),
        ];
        sections.extend(self.sections.iter().cloned());
        sections.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.id.cmp(right.id))
        });
        sections.into_iter().map(|section| section.body).collect()
    }

    #[must_use]
    pub fn fingerprint(&self, ctx: &PromptCtx<'_>) -> [u8; 32] {
        Sha256::digest(self.build(ctx).join("\n").as_bytes()).into()
    }
}

fn role_section(ctx: &PromptCtx<'_>) -> SystemPromptSection {
    SystemPromptSection {
        id: "role",
        order: 10,
        body: format!(
            "<role>\nYou are the Octopus SDK runtime assistant.\nSession: {}\nPermission mode: {}\nProject root: {}\n</role>",
            ctx.session.0,
            mode_label(ctx.mode),
            ctx.project_root.display()
        ),
    }
}

fn tools_guidance_section(surface: &ToolSurface) -> SystemPromptSection {
    let mut lines = vec![
        "<tools_guidance>".to_string(),
        "Use tools just in time. Prefer reading before writing.".to_string(),
    ];
    lines.extend(surface.prompt_lines());
    lines.push("</tools_guidance>".to_string());

    SystemPromptSection {
        id: "tools_guidance",
        order: 20,
        body: lines.join("\n"),
    }
}

fn process_section() -> SystemPromptSection {
    SystemPromptSection {
        id: "process",
        order: 30,
        body: "<process>\nExplore -> Plan -> Implement -> Verify.\nKeep edits deterministic.\n</process>"
            .into(),
    }
}

fn safety_section() -> SystemPromptSection {
    SystemPromptSection {
        id: "safety",
        order: 40,
        body: "<safety>\nRead before mutate.\nDo not serialize secrets into logs.\n</safety>"
            .into(),
    }
}

fn output_section() -> SystemPromptSection {
    SystemPromptSection {
        id: "output",
        order: 50,
        body: "<output>\nUse concise markdown.\nReference real files when relevant.\n</output>"
            .into(),
    }
}

fn mode_label(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "default",
        PermissionMode::AcceptEdits => "accept_edits",
        PermissionMode::BypassPermissions => "bypass_permissions",
        PermissionMode::DontAsk => "dont_ask",
        PermissionMode::Auto => "auto",
        PermissionMode::Bubble => "bubble",
        PermissionMode::Plan => "plan",
    }
}
