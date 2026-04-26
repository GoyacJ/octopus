//! Dangerous command pattern library.

use regex::Regex;

#[derive(Debug, Clone)]
pub struct DangerousPatternLibrary {
    rules: Vec<DangerousPatternRule>,
}

impl DangerousPatternLibrary {
    pub fn new(rules: Vec<DangerousPatternRule>) -> Self {
        Self { rules }
    }

    pub fn default_unix() -> Self {
        Self::new(
            [
                ("unix-rm-root", r"(?i)\brm\s+-[^\n]*r[^\n]*\s+/(?:\s|$)"),
                (
                    "unix-rm-force-recursive",
                    r"(?i)\brm\s+-[^\n]*rf[^\n]*\s+(?:/|~|/etc|/usr|/var|/System)",
                ),
                ("unix-dd-disk", r"(?i)\bdd\s+.*\bof=/dev/"),
                ("unix-mkfs", r"(?i)\bmkfs(?:\.[a-z0-9]+)?\b"),
                ("unix-shred", r"(?i)\bshred\b"),
                ("unix-wipefs", r"(?i)\bwipefs\b"),
                ("unix-chmod-777-root", r"(?i)\bchmod\s+-R\s+777\s+/"),
                ("unix-chown-root", r"(?i)\bchown\s+-R\b.*\s+/"),
                ("unix-fork-bomb", r":\(\)\s*\{\s*:\|:\s*&\s*\}\s*;:"),
                (
                    "unix-dev-random-disk",
                    r"(?i)/dev/(zero|random|urandom).*>/dev/",
                ),
                ("unix-sudo-rm", r"(?i)\bsudo\s+rm\s+-"),
                (
                    "unix-curl-shell",
                    r"(?i)\b(curl|wget)\b.*\|\s*(sh|bash|zsh)\b",
                ),
                ("unix-kill-all", r"(?i)\bkillall\s+-9\b"),
                ("unix-pkill-all", r"(?i)\bpkill\s+-9\b"),
                ("unix-iptables-flush", r"(?i)\biptables\s+-F\b"),
                ("unix-nft-flush", r"(?i)\bnft\s+flush\s+ruleset\b"),
                ("unix-systemctl-stop", r"(?i)\bsystemctl\s+(stop|disable)\b"),
                ("unix-launchctl-unload", r"(?i)\blaunchctl\s+unload\b"),
                ("unix-crontab-remove", r"(?i)\bcrontab\s+-r\b"),
                ("unix-find-delete-root", r"(?i)\bfind\s+/\s+.*-delete\b"),
            ]
            .into_iter()
            .map(|(id, pattern)| DangerousPatternRule::new(id, pattern))
            .collect(),
        )
    }

    pub fn default_windows() -> Self {
        Self::new(
            [
                ("windows-format", r"(?i)\bformat\s+[a-z]:"),
                (
                    "windows-del-system",
                    r"(?i)\bdel\s+/[^\n]*[sq][^\n]*\s+c:\\",
                ),
                (
                    "windows-rmdir-system",
                    r"(?i)\brmdir\s+/[^\n]*[sq][^\n]*\s+c:\\",
                ),
                ("windows-remove-item-c", r"(?i)\bremove-item\b.*\bc:\\"),
                ("windows-stop-service", r"(?i)\bstop-service\b"),
                ("windows-disable-service", r"(?i)\bset-service\b.*disabled"),
                ("windows-bcdedit", r"(?i)\bbcdedit\b"),
                ("windows-reg-delete", r"(?i)\breg\s+delete\b"),
                ("windows-taskkill-all", r"(?i)\btaskkill\s+/f\b"),
                ("windows-diskpart", r"(?i)\bdiskpart\b"),
                ("windows-clear-eventlog", r"(?i)\bwevtutil\s+cl\b"),
                ("windows-powershell-iex", r"(?i)\biex\s*\("),
                (
                    "windows-download-exec",
                    r"(?i)\binvoke-webrequest\b.*\|\s*iex\b",
                ),
            ]
            .into_iter()
            .map(|(id, pattern)| DangerousPatternRule::new(id, pattern))
            .collect(),
        )
    }

    pub fn default_all() -> Self {
        let mut rules = Self::default_unix().rules;
        rules.extend(Self::default_windows().rules);
        Self::new(rules)
    }

    pub fn detect(&self, command: &str) -> Vec<&DangerousPatternRule> {
        self.rules
            .iter()
            .filter(|rule| rule.matches(command))
            .collect()
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
    id: String,
    pattern: String,
    regex: Regex,
}

impl DangerousPatternRule {
    pub fn new(id: impl Into<String>, pattern: impl Into<String>) -> Self {
        let pattern = pattern.into();
        let regex = Regex::new(&pattern).expect("dangerous pattern regex should compile");
        Self {
            id: id.into(),
            pattern,
            regex,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn matches(&self, command: &str) -> bool {
        self.regex.is_match(command)
    }
}
