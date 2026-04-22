use super::{
    render_slash_command_help, suggest_slash_commands, validate_slash_command_input, SlashCommand,
};

#[test]
fn parses_plugins_and_skills_commands() {
    assert_eq!(
        SlashCommand::parse("/plugins install demo"),
        Ok(Some(SlashCommand::Plugins {
            action: Some("install".into()),
            target: Some("demo".into()),
        }))
    );
    assert_eq!(
        SlashCommand::parse("/skills install ./fixtures/help-skill"),
        Ok(Some(SlashCommand::Skills {
            args: Some("install ./fixtures/help-skill".into()),
        }))
    );
}

#[test]
fn renders_help_and_suggestions() {
    let help = render_slash_command_help();
    assert!(help.contains("/agents"));
    assert_eq!(
        suggest_slash_commands("/hep", 2).first(),
        Some(&"/help".to_string())
    );
}

#[test]
fn rejects_invalid_clear_args() {
    let error = validate_slash_command_input("/clear nope").expect_err("invalid args should fail");
    assert!(error.to_string().contains("/clear [--confirm]"));
}
