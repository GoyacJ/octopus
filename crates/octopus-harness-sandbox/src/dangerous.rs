//! Dangerous command pattern library shared by sandbox and permission layers.

use harness_contracts::{
    default_unix_dangerous_pattern_specs, default_windows_dangerous_pattern_specs,
    DangerousPatternSpec, Severity,
};
use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone)]
pub struct DangerousPatternLibrary {
    rules: Vec<DangerousPatternRule>,
}

impl DangerousPatternLibrary {
    pub fn default_unix() -> Self {
        Self {
            rules: compile_rules(default_unix_dangerous_pattern_specs()),
        }
    }

    pub fn default_windows() -> Self {
        Self {
            rules: compile_rules(default_windows_dangerous_pattern_specs()),
        }
    }

    pub fn default_all() -> Self {
        let mut rules = compile_rules(default_unix_dangerous_pattern_specs());
        rules.extend(compile_rules(default_windows_dangerous_pattern_specs()));
        Self { rules }
    }

    pub fn detect(&self, command: &str) -> Option<&DangerousPatternRule> {
        self.rules
            .iter()
            .find(|rule| rule.pattern.is_match(command))
    }

    pub fn rules(&self) -> &[DangerousPatternRule] {
        &self.rules
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct DangerousPatternRule {
    pub id: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub description: String,
}

fn compile_rules(specs: &[DangerousPatternSpec]) -> Vec<DangerousPatternRule> {
    specs
        .iter()
        .map(|spec| DangerousPatternRule {
            id: spec.id.to_owned(),
            pattern: RegexBuilder::new(spec.pattern)
                .case_insensitive(true)
                .build()
                .expect("dangerous pattern regex should compile"),
            severity: spec.severity,
            description: spec.description.to_owned(),
        })
        .collect()
}
