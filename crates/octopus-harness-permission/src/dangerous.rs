use harness_contracts::{
    default_unix_dangerous_pattern_specs, default_windows_dangerous_pattern_specs,
    DangerousPatternSpec, Severity, ShellKind,
};
use regex::{Regex, RegexBuilder};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub struct DangerousPatternLibrary {
    patterns: Vec<DangerousPatternRule>,
}

#[derive(Debug, Clone)]
pub struct DangerousPatternRule {
    pub id: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub description: String,
}

impl DangerousPatternLibrary {
    pub fn default_unix() -> Self {
        Self {
            patterns: compile_patterns(default_unix_dangerous_pattern_specs()),
        }
    }

    pub fn default_windows() -> Self {
        Self {
            patterns: compile_patterns(default_windows_dangerous_pattern_specs()),
        }
    }

    pub fn default_all() -> Self {
        let mut patterns = compile_patterns(default_unix_dangerous_pattern_specs());
        patterns.extend(compile_patterns(default_windows_dangerous_pattern_specs()));
        Self { patterns }
    }

    pub fn for_shell_kind(shell_kind: ShellKind) -> Self {
        match shell_kind {
            ShellKind::Bash(_) | ShellKind::Zsh(_) => Self::default_unix(),
            ShellKind::PowerShell => Self::default_windows(),
            ShellKind::System | _ => Self::default_all(),
        }
    }

    pub fn detect(&self, command: &str) -> Option<&DangerousPatternRule> {
        let normalized = normalize_command_for_detection(command);
        self.patterns
            .iter()
            .find(|rule| rule.pattern.is_match(&normalized))
    }

    pub fn patterns(&self) -> &[DangerousPatternRule] {
        &self.patterns
    }
}

fn compile_patterns(specs: &[DangerousPatternSpec]) -> Vec<DangerousPatternRule> {
    specs
        .iter()
        .map(|spec| DangerousPatternRule {
            id: spec.id.to_owned(),
            pattern: RegexBuilder::new(spec.pattern)
                .case_insensitive(true)
                .build()
                .expect("builtin dangerous pattern should compile"),
            severity: spec.severity,
            description: spec.description.to_owned(),
        })
        .collect()
}

fn normalize_command_for_detection(command: &str) -> String {
    let stripped = strip_ansi_escapes::strip_str(command);
    stripped.nfkc().collect()
}
