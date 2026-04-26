use harness_contracts::{Severity, ShellKind};
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
            patterns: unix_patterns(),
        }
    }

    pub fn default_windows() -> Self {
        Self {
            patterns: windows_patterns(),
        }
    }

    pub fn default_all() -> Self {
        let mut patterns = unix_patterns();
        patterns.extend(windows_patterns());
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

fn unix_patterns() -> Vec<DangerousPatternRule> {
    vec![
        rule(
            "unix-rm-rf-root",
            r"\brm\s+-[^|;&]*r[^|;&]*f[^|;&]*\s+(/|~)(\s|$)",
            "recursive force delete of root or home",
        ),
        rule(
            "unix-chmod-777-root",
            r"\bchmod\s+-?R?\s+777\s+/(?:\s|$)",
            "world-writable root permissions",
        ),
        rule(
            "unix-chown-recursive-root",
            r"\bchown\s+-R\s+[^ ]+\s+/(?:\s|$)",
            "recursive root ownership rewrite",
        ),
        rule(
            "unix-curl-pipe-shell",
            r"\b(curl|wget)\b[^|;&]*\|\s*(sh|bash|zsh)\b",
            "downloaded script piped to shell",
        ),
        rule(
            "unix-fork-bomb",
            r":\(\)\s*\{\s*:\|:\s*&\s*\}\s*;:",
            "shell fork bomb",
        ),
        rule(
            "unix-shutdown",
            r"\b(shutdown|reboot|halt|poweroff)\b",
            "host shutdown command",
        ),
        rule(
            "unix-dd-disk",
            r"\bdd\b[^|;&]*(of=/dev/(sd|disk|nvme|rdisk)|if=/dev/zero)",
            "raw disk overwrite",
        ),
        rule(
            "unix-mkfs",
            r"\bmkfs(\.[a-z0-9]+)?\b",
            "filesystem creation over device",
        ),
        rule(
            "unix-diskutil-erase",
            r"\bdiskutil\s+eraseDisk\b",
            "macOS disk erase",
        ),
        rule(
            "unix-sudoers-write",
            r"\b(tee|cat|echo)\b[^|;&]*(/etc/sudoers|/etc/sudoers\.d/)",
            "sudoers mutation",
        ),
        rule(
            "unix-passwd-shadow-write",
            r"\b(tee|cat|echo|sed)\b[^|;&]*(/etc/passwd|/etc/shadow)",
            "account database mutation",
        ),
        rule(
            "unix-ssh-key-delete",
            r"\brm\s+-[^|;&]*f[^|;&]*\s+[^|;&]*(\.ssh/(id_|authorized_keys|known_hosts))",
            "ssh credential deletion",
        ),
        rule(
            "git-force-push-main",
            r"\bgit\s+push\b[^|;&]*(--force|-f|--mirror)[^|;&]*(main|master)\b",
            "force push protected branch",
        ),
        rule(
            "unix-kill-all",
            r"\b(killall|pkill)\s+(-9\s+)?(-u\s+\S+|.*)",
            "broad process termination",
        ),
        rule(
            "unix-crontab-persistence",
            r"\b(crontab\s+|/etc/cron\.|/var/spool/cron)",
            "cron persistence mutation",
        ),
        rule(
            "unix-launchctl-persistence",
            r"\blaunchctl\s+(load|bootstrap|enable)\b",
            "macOS launch agent persistence",
        ),
    ]
}

fn windows_patterns() -> Vec<DangerousPatternRule> {
    vec![
        rule(
            "windows-remove-item-recurse-root",
            r"\bremove-item\b[^|;&]*(-recurse|-r)[^|;&]*(-force|-fo)[^|;&]*([a-z]:\\|c:\\)",
            "recursive force delete of drive root",
        ),
        rule(
            "windows-format-volume",
            r"\b(format-volume|format\s+[a-z]:)\b",
            "format Windows volume",
        ),
        rule(
            "windows-stop-computer",
            r"\b(stop-computer|restart-computer|shutdown\.exe)\b",
            "Windows shutdown command",
        ),
        rule(
            "windows-diskpart-clean",
            r"\bdiskpart\b[^|;&]*\bclean\b",
            "diskpart clean operation",
        ),
        rule(
            "windows-iwr-iex",
            r"\b(iwr|irm|invoke-webrequest|invoke-restmethod)\b[^|;&]*\|\s*(iex|invoke-expression)\b",
            "downloaded script piped to PowerShell",
        ),
        rule(
            "windows-webclient-iex",
            r"downloadstring\s*\([^)]*\)\s*\|\s*(iex|invoke-expression)\b",
            "WebClient downloaded script execution",
        ),
        rule(
            "windows-cmd-del-root",
            r"\bdel\b[^|;&]*(/s|/q)[^|;&]*[a-z]:\\",
            "recursive cmd delete of drive",
        ),
        rule(
            "windows-rmdir-root",
            r"\brmdir\b[^|;&]*(/s|/q)[^|;&]*[a-z]:\\",
            "recursive cmd rmdir of drive",
        ),
        rule(
            "windows-executionpolicy-unrestricted",
            r"\bset-executionpolicy\b[^|;&]*(unrestricted|bypass)\b",
            "weaken PowerShell execution policy",
        ),
        rule(
            "windows-disable-defender",
            r"\bset-mppreference\b[^|;&]*-disablerealtimemonitoring\b",
            "disable Defender realtime monitoring",
        ),
        rule(
            "windows-add-trusted-publisher",
            r"\bcertutil\b[^|;&]*-addstore[^|;&]*(trustedpublisher|root)\b",
            "add trusted certificate publisher",
        ),
        rule(
            "windows-registry-run-key",
            r"\b(reg\s+add|new-itemproperty)\b[^|;&]*(\\software\\microsoft\\windows\\currentversion\\run|currentversion\\run)",
            "Windows run key persistence",
        ),
        rule(
            "windows-bcdedit-disable-recovery",
            r"\bbcdedit\b[^|;&]*(recoveryenabled\s+no|bootstatuspolicy\s+ignoreallfailures)",
            "disable boot recovery",
        ),
        rule(
            "windows-vss-delete",
            r"\bvssadmin\b[^|;&]*delete\s+shadows\b",
            "delete shadow copies",
        ),
        rule(
            "windows-wevtutil-clear",
            r"\bwevtutil\b[^|;&]*\bcl\b",
            "clear Windows event logs",
        ),
    ]
}

fn rule(id: &str, pattern: &str, description: &str) -> DangerousPatternRule {
    DangerousPatternRule {
        id: id.to_owned(),
        pattern: RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .expect("builtin dangerous pattern should compile"),
        severity: Severity::Critical,
        description: description.to_owned(),
    }
}

fn normalize_command_for_detection(command: &str) -> String {
    let stripped = strip_ansi_escapes::strip_str(command);
    stripped.nfkc().collect()
}
