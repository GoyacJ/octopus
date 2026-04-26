use std::ops::Range;

use harness_contracts::{MemoryError, Severity, ThreatAction, ThreatCategory};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct MemoryThreatScanner {
    patterns: Vec<ThreatPattern>,
}

#[derive(Debug, Clone)]
pub struct ThreatPattern {
    pub id: String,
    pub expression: String,
    pub category: ThreatCategory,
    pub severity: Severity,
    pub action: ThreatAction,
    regex: Regex,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ThreatScanReport {
    pub action: ThreatAction,
    pub hits: Vec<ThreatHit>,
    pub redacted_content: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ThreatHit {
    pub pattern_id: String,
    pub category: ThreatCategory,
    pub severity: Severity,
    pub action: ThreatAction,
    pub range: Range<usize>,
}

#[derive(Debug, Clone, Copy)]
struct DefaultPatternSpec {
    id: &'static str,
    expression: &'static str,
    category: ThreatCategory,
    severity: Severity,
    action: ThreatAction,
}

impl ThreatPattern {
    pub fn new(
        id: impl Into<String>,
        expression: impl Into<String>,
        category: ThreatCategory,
        severity: Severity,
        action: ThreatAction,
    ) -> Result<Self, MemoryError> {
        let id = id.into();
        let expression = expression.into();
        let regex = Regex::new(&expression).map_err(|error| {
            MemoryError::Message(format!("invalid threat pattern {id}: {error}"))
        })?;

        Ok(Self {
            id,
            expression,
            category,
            severity,
            action,
            regex,
        })
    }
}

impl MemoryThreatScanner {
    #[must_use]
    pub fn from_patterns(patterns: Vec<ThreatPattern>) -> Self {
        Self { patterns }
    }

    #[must_use]
    pub fn patterns(&self) -> &[ThreatPattern] {
        &self.patterns
    }

    #[must_use]
    pub fn scan(&self, content: &str) -> ThreatScanReport {
        let mut action = ThreatAction::Warn;
        let mut hits = Vec::new();

        for pattern in &self.patterns {
            for found in pattern.regex.find_iter(content) {
                action = strongest_action(action, pattern.action);
                hits.push(ThreatHit {
                    pattern_id: pattern.id.clone(),
                    category: pattern.category,
                    severity: pattern.severity,
                    action: pattern.action,
                    range: found.start()..found.end(),
                });
            }
        }

        let redacted_content = if action == ThreatAction::Redact {
            redact_content(content, &hits)
        } else {
            None
        };

        ThreatScanReport {
            action,
            hits,
            redacted_content,
        }
    }
}

impl Default for MemoryThreatScanner {
    fn default() -> Self {
        let patterns = DEFAULT_PATTERN_SPECS
            .iter()
            .map(|spec| {
                ThreatPattern::new(
                    spec.id,
                    spec.expression,
                    spec.category,
                    spec.severity,
                    spec.action,
                )
                .expect("default threat pattern must compile")
            })
            .collect();

        Self { patterns }
    }
}

fn strongest_action(left: ThreatAction, right: ThreatAction) -> ThreatAction {
    if action_rank(right) > action_rank(left) {
        right
    } else {
        left
    }
}

fn action_rank(action: ThreatAction) -> u8 {
    if action == ThreatAction::Block {
        2
    } else if action == ThreatAction::Redact {
        1
    } else {
        0
    }
}

fn redact_content(content: &str, hits: &[ThreatHit]) -> Option<String> {
    let mut ranges = hits
        .iter()
        .filter(|hit| hit.action == ThreatAction::Redact)
        .map(|hit| (hit.range.clone(), hit.category))
        .collect::<Vec<_>>();

    if ranges.is_empty() {
        return None;
    }

    ranges.sort_by_key(|(range, _)| (range.start, range.end));

    let mut merged: Vec<(Range<usize>, ThreatCategory)> = Vec::new();
    for (range, category) in ranges {
        if let Some((last_range, _)) = merged.last_mut() {
            if range.start <= last_range.end {
                last_range.end = last_range.end.max(range.end);
                continue;
            }
        }

        merged.push((range, category));
    }

    let mut out = String::with_capacity(content.len());
    let mut cursor = 0;
    for (range, category) in merged {
        out.push_str(&content[cursor..range.start]);
        out.push_str("[REDACTED:");
        out.push_str(category_label(category));
        out.push(']');
        cursor = range.end;
    }
    out.push_str(&content[cursor..]);

    Some(out)
}

fn category_label(category: ThreatCategory) -> &'static str {
    match category {
        ThreatCategory::PromptInjection => "prompt_injection",
        ThreatCategory::Exfiltration => "exfiltration",
        ThreatCategory::Backdoor => "backdoor",
        ThreatCategory::Credential => "credential",
        ThreatCategory::Malicious => "malicious",
        ThreatCategory::SpecialToken => "special_token",
        _ => "unknown",
    }
}

const DEFAULT_PATTERN_SPECS: &[DefaultPatternSpec] = &[
    DefaultPatternSpec {
        id: "prompt_injection_ignore_previous",
        expression: r"(?i)\bignore\s+(?:all\s+)?previous\s+instructions?\b",
        category: ThreatCategory::PromptInjection,
        severity: Severity::Critical,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "prompt_injection_reveal_system_prompt",
        expression: r"(?i)\breveal\s+(?:the\s+)?system\s+prompt\b",
        category: ThreatCategory::PromptInjection,
        severity: Severity::Critical,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "prompt_injection_disregard_developer",
        expression: r"(?i)\bdisregard\s+(?:the\s+)?(?:developer|system)\s+message\b",
        category: ThreatCategory::PromptInjection,
        severity: Severity::High,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "prompt_injection_override_rules",
        expression: r"(?i)\boverride\s+(?:all\s+)?(?:safety\s+)?rules\b",
        category: ThreatCategory::PromptInjection,
        severity: Severity::High,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "prompt_injection_jailbreak",
        expression: r"(?i)\bjailbreak\s+mode\b",
        category: ThreatCategory::PromptInjection,
        severity: Severity::High,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "exfiltration_send_secrets",
        expression: r"(?i)\bsend\s+(?:all\s+)?(?:secrets|credentials|tokens)\s+to\b",
        category: ThreatCategory::Exfiltration,
        severity: Severity::Critical,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "exfiltration_post_env",
        expression: r"(?i)\bpost\s+(?:the\s+)?(?:env|environment)\s+(?:vars|variables)\b",
        category: ThreatCategory::Exfiltration,
        severity: Severity::Critical,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "exfiltration_curl_secret",
        expression: r"(?i)\bcurl\b.{0,80}\b(?:secret|token|credential)s?\b",
        category: ThreatCategory::Exfiltration,
        severity: Severity::High,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "exfiltration_upload_ssh",
        expression: r"(?i)\bupload\b.{0,80}\b\.ssh\b",
        category: ThreatCategory::Exfiltration,
        severity: Severity::High,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "exfiltration_print_private_key",
        expression: r"(?i)\bprint\s+(?:the\s+)?private\s+key\b",
        category: ThreatCategory::Exfiltration,
        severity: Severity::Critical,
        action: ThreatAction::Block,
    },
    DefaultPatternSpec {
        id: "backdoor_cron_persistence",
        expression: r"(?i)\bcron\b.{0,80}\b(?:backdoor|persistence)\b",
        category: ThreatCategory::Backdoor,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "backdoor_authorized_keys",
        expression: r"(?i)\bauthorized_keys\b.{0,80}\b(?:append|add|inject)\b",
        category: ThreatCategory::Backdoor,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "backdoor_persistence_agent",
        expression: r"(?i)\bpersistence\s+(?:agent|daemon|service)\b",
        category: ThreatCategory::Backdoor,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "backdoor_hidden_admin",
        expression: r"(?i)\bhidden\s+(?:admin|administrator|user)\b",
        category: ThreatCategory::Backdoor,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "backdoor_startup_script",
        expression: r"(?i)\bstartup\s+script\b.{0,80}\b(?:payload|persistence)\b",
        category: ThreatCategory::Backdoor,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "credential_api_key",
        expression: r"(?i)\bapi[_-]?key\s*[:=]\s*[A-Za-z0-9_\-]{16,}\b",
        category: ThreatCategory::Credential,
        severity: Severity::High,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "credential_secret_assignment",
        expression: r"(?i)\bsecret\s*[:=]\s*[A-Za-z0-9_\-]{8,}\b",
        category: ThreatCategory::Credential,
        severity: Severity::High,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "credential_bearer_token",
        expression: r"(?i)\bbearer\s+[A-Za-z0-9._\-]{20,}\b",
        category: ThreatCategory::Credential,
        severity: Severity::High,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "credential_private_key_block",
        expression: r"-----BEGIN\s+(?:RSA\s+|OPENSSH\s+|EC\s+)?PRIVATE\s+KEY-----",
        category: ThreatCategory::Credential,
        severity: Severity::Critical,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "credential_password_assignment",
        expression: r"(?i)\bpassword\s*[:=]\s*[^\s]{8,}",
        category: ThreatCategory::Credential,
        severity: Severity::Medium,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "malicious_reverse_shell",
        expression: r"(?i)\breverse\s+shell\b",
        category: ThreatCategory::Malicious,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "malicious_rm_rf_root",
        expression: r"(?i)\brm\s+-rf\s+/\b",
        category: ThreatCategory::Malicious,
        severity: Severity::Critical,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "malicious_netcat_exec",
        expression: r"(?i)\bnc\s+-e\s+/bin/(?:sh|bash)\b",
        category: ThreatCategory::Malicious,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "malicious_chmod_suid",
        expression: r"(?i)\bchmod\s+u\+s\b",
        category: ThreatCategory::Malicious,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "malicious_shell_payload",
        expression: r"(?i)\b(?:bash|sh)\s+-i\s+>&\s*/dev/tcp/",
        category: ThreatCategory::Malicious,
        severity: Severity::High,
        action: ThreatAction::Warn,
    },
    DefaultPatternSpec {
        id: "special_token_memory_context_open",
        expression: r"(?i)<memory-context>",
        category: ThreatCategory::SpecialToken,
        severity: Severity::Medium,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "special_token_memory_context_close",
        expression: r"(?i)</memory-context>",
        category: ThreatCategory::SpecialToken,
        severity: Severity::Medium,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "special_token_system_open",
        expression: r"(?i)<system>",
        category: ThreatCategory::SpecialToken,
        severity: Severity::Medium,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "special_token_system_close",
        expression: r"(?i)</system>",
        category: ThreatCategory::SpecialToken,
        severity: Severity::Medium,
        action: ThreatAction::Redact,
    },
    DefaultPatternSpec {
        id: "special_token_tool_result",
        expression: r"(?i)</?tool_result>",
        category: ThreatCategory::SpecialToken,
        severity: Severity::Low,
        action: ThreatAction::Redact,
    },
];
