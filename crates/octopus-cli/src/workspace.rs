mod discovery;
mod handlers;
mod install;
mod render;
#[cfg(test)]
mod tests;
mod types;

pub use handlers::{
    handle_agents_slash_command, handle_agents_slash_command_json, handle_skills_slash_command,
    handle_skills_slash_command_json,
};
pub use types::InstalledSkill;

pub(crate) use discovery::{
    discover_definition_roots, discover_skill_roots, load_agents_from_roots,
    load_skills_from_roots, parse_skill_frontmatter,
};
pub(crate) use install::install_skill;
pub(crate) use render::{
    render_agents_report, render_agents_report_json, render_agents_usage, render_agents_usage_json,
    render_skill_install_report, render_skill_install_report_json, render_skills_report,
    render_skills_report_json, render_skills_usage, render_skills_usage_json,
};
pub(crate) use types::{
    AgentSummary, DefinitionScope, DefinitionSource, SkillInstallSource, SkillOrigin, SkillRoot,
    SkillSummary,
};
