use super::{handle_agents_slash_command, handle_skills_slash_command, parse_skill_frontmatter};

#[test]
fn parses_skill_frontmatter() {
    let (name, description) =
        parse_skill_frontmatter("---\nname: Test\ndescription: Demo\n---\nbody");
    assert_eq!(name.as_deref(), Some("Test"));
    assert_eq!(description.as_deref(), Some("Demo"));
}

#[test]
fn renders_help_for_workspace_commands() {
    let cwd = tempfile::tempdir().expect("tempdir should exist");
    assert!(handle_agents_slash_command(Some("help"), cwd.path())
        .expect("agents help")
        .contains("Usage"));
    assert!(handle_skills_slash_command(Some("help"), cwd.path())
        .expect("skills help")
        .contains("Usage"));
}
