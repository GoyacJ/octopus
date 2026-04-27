use harness_contracts::McpServerId;

use crate::{
    parse_skill_markdown, LoadReport, SkillError, SkillPlatform, SkillRejectReason, SkillRejection,
    SkillSource,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpSkillRecord {
    pub name: String,
    pub description: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct McpSource {
    server_id: McpServerId,
    records: Vec<McpSkillRecord>,
}

impl McpSource {
    #[must_use]
    pub fn new(server_id: McpServerId, records: Vec<McpSkillRecord>) -> Self {
        Self { server_id, records }
    }

    pub async fn load(&self, runtime_platform: SkillPlatform) -> Result<LoadReport, SkillError> {
        let mut loaded = Vec::new();
        let mut rejected = Vec::new();
        for record in &self.records {
            let source = SkillSource::Mcp(self.server_id.clone());
            let markdown = format!(
                "---\nname: {}\ndescription: {}\n---\n{}",
                canonical_mcp_skill_name(&self.server_id, &record.name),
                record.description,
                record.body
            );
            match parse_skill_markdown(&markdown, source.clone(), None, runtime_platform) {
                Ok(skill) => loaded.push(skill),
                Err(error) => rejected.push(SkillRejection {
                    source,
                    raw_path: None,
                    reason: SkillRejectReason::from_error(&error),
                }),
            }
        }

        Ok(LoadReport { loaded, rejected })
    }
}

fn canonical_mcp_skill_name(server_id: &McpServerId, skill_name: &str) -> String {
    format!("mcp__{}__{}", server_id.0, skill_name)
}
