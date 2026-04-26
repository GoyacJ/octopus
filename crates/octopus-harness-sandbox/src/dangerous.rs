//! Dangerous command pattern library shared by sandbox and permission layers.

use harness_contracts::Severity;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct DangerousPatternLibrary {
    rules: Vec<DangerousPatternRule>,
}

impl DangerousPatternLibrary {
    pub fn default_unix() -> Self {
        Self {
            rules: vec![
                rule(
                    "unix_rm_rf_root",
                    r"(?i)(^|[;&|]\s*)rm\s+-[^\s]*r[^\s]*f[^\s]*\s+(?:/|/\*)(?:\s|$|[;&|])",
                    Severity::Critical,
                    "recursive root removal",
                ),
                rule(
                    "unix_sudo_rm_rf",
                    r"(?i)\bsudo\s+rm\s+-[^\s]*r[^\s]*f[^\s]*\s+/",
                    Severity::Critical,
                    "privileged recursive removal",
                ),
                rule(
                    "unix_dd_over_device",
                    r"(?i)\bdd\b.*\bof=/dev/[^\s]+",
                    Severity::Critical,
                    "raw device overwrite",
                ),
                rule(
                    "unix_mkfs_device",
                    r"(?i)\bmkfs(?:\.[a-z0-9]+)?\s+/dev/[^\s]+",
                    Severity::Critical,
                    "filesystem formatting",
                ),
                rule(
                    "unix_fork_bomb",
                    r"\:\(\)\{\s*\:\|\:\&\s*\}\;\:",
                    Severity::Critical,
                    "shell fork bomb",
                ),
                rule(
                    "unix_chmod_777_root",
                    r"(?i)\bchmod\b.*\b777\b\s+(?:/|/\*)(?:\s|$)",
                    Severity::High,
                    "world-writable root permissions",
                ),
                rule(
                    "unix_chown_recursive_root",
                    r"(?i)\bchown\b\s+-R\b.*\s+(?:/|/\*)(?:\s|$)",
                    Severity::High,
                    "recursive root ownership change",
                ),
                rule(
                    "unix_curl_pipe_sh",
                    r"(?i)\bcurl\b[^|]*\|\s*(?:sh|bash|zsh)\b",
                    Severity::High,
                    "download piped to shell",
                ),
                rule(
                    "unix_wget_pipe_bash",
                    r"(?i)\bwget\b[^|]*\|\s*(?:sh|bash|zsh)\b",
                    Severity::High,
                    "download piped to shell",
                ),
                rule(
                    "git_force_push_main",
                    r"(?i)\bgit\s+push\b.*(?:--force|-f)\b.*\bmain\b",
                    Severity::High,
                    "force push to main",
                ),
                rule(
                    "git_reset_hard_previous",
                    r"(?i)\bgit\s+reset\s+--hard\s+HEAD~[0-9]*\b",
                    Severity::High,
                    "destructive git reset",
                ),
                rule(
                    "unix_modify_sudoers",
                    r"(?i)(/etc/sudoers|\bvisudo\b)",
                    Severity::Critical,
                    "sudoers modification",
                ),
                rule(
                    "unix_authorized_keys_write",
                    r"(?i)(>>|>)\s*(?:~|\$HOME)?/?.*\.ssh/authorized_keys\b",
                    Severity::High,
                    "ssh authorized_keys persistence",
                ),
                rule(
                    "unix_shell_profile_persistence",
                    r"(?i)(>>|>)\s*(?:~|\$HOME)?/?\.(?:bashrc|zshrc|profile|bash_profile)\b",
                    Severity::High,
                    "shell profile persistence",
                ),
                rule(
                    "unix_read_aws_credentials",
                    r"(?i)(?:cat|less|more|grep)\b.*(?:~|\$HOME)?/?\.aws/credentials\b",
                    Severity::High,
                    "cloud credential read",
                ),
                rule(
                    "unix_env_dump",
                    r"(?i)(^|[;&|]\s*)env\s*>\s*/tmp/[^\s]+",
                    Severity::Medium,
                    "environment dump",
                ),
                rule(
                    "unix_clear_history",
                    r"(?i)\bhistory\s+-c\b",
                    Severity::Medium,
                    "shell history clearing",
                ),
                rule(
                    "unix_killall_force",
                    r"(?i)\bkillall\s+-9\b",
                    Severity::High,
                    "force kill by process name",
                ),
            ],
        }
    }

    pub fn default_windows() -> Self {
        Self {
            rules: vec![
                rule(
                    "windows_remove_item_root",
                    r"(?i)\bremove-item\b.*(?:^|\s)-recurse\b.*(?:^|\s)-force\b.*\bc:\\(?:\s|$)",
                    Severity::Critical,
                    "recursive root removal",
                ),
                rule(
                    "windows_format_volume",
                    r"(?i)\bformat-volume\b",
                    Severity::Critical,
                    "volume formatting",
                ),
                rule(
                    "windows_stop_computer",
                    r"(?i)\bstop-computer\b",
                    Severity::Critical,
                    "host shutdown",
                ),
                rule(
                    "windows_restart_computer",
                    r"(?i)\brestart-computer\b",
                    Severity::Critical,
                    "host restart",
                ),
                rule(
                    "windows_diskpart_clean",
                    r"(?i)\bdiskpart\b.*\bclean\b",
                    Severity::Critical,
                    "disk cleanup",
                ),
                rule(
                    "windows_iwr_iex",
                    r"(?i)\b(?:iwr|invoke-webrequest)\b[^|]*\|\s*(?:iex|invoke-expression)\b",
                    Severity::High,
                    "download piped to PowerShell execution",
                ),
                rule(
                    "windows_downloadstring_iex",
                    r"(?i)\bdownloadstring\b[^|]*\|\s*(?:iex|invoke-expression)\b",
                    Severity::High,
                    "download string executed",
                ),
                rule(
                    "windows_del_root",
                    r"(?i)\bdel\b.*(?:/s\b.* /q\b|/q\b.* /s\b|/s\b.*?/q\b|/q\b.*?/s\b).*\bc:\\(?:\s|$)",
                    Severity::Critical,
                    "recursive root delete",
                ),
                rule(
                    "windows_rmdir_root",
                    r"(?i)\brmdir\b.*(?:/s\b.* /q\b|/q\b.* /s\b|/s\b.*?/q\b|/q\b.*?/s\b).*\bc:\\(?:\s|$)",
                    Severity::Critical,
                    "recursive root directory delete",
                ),
                rule(
                    "windows_execution_policy_unrestricted",
                    r"(?i)\bset-executionpolicy\b.*\b(?:unrestricted|bypass)\b",
                    Severity::High,
                    "execution policy weakening",
                ),
                rule(
                    "windows_disable_defender",
                    r"(?i)\bset-mppreference\b.*(?:^|\s)-disablerealtimemonitoring\b",
                    Severity::Critical,
                    "defender realtime protection disabled",
                ),
                rule(
                    "windows_registry_run_key",
                    r"(?i)\breg\s+add\b.*\\currentversion\\run\b",
                    Severity::High,
                    "registry run key persistence",
                ),
            ],
        }
    }

    pub fn default_all() -> Self {
        let mut rules = Self::default_unix().rules;
        rules.extend(Self::default_windows().rules);
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

fn rule(id: &str, pattern: &str, severity: Severity, description: &str) -> DangerousPatternRule {
    DangerousPatternRule {
        id: id.to_owned(),
        pattern: Regex::new(pattern).expect("dangerous pattern regex should compile"),
        severity,
        description: description.to_owned(),
    }
}
