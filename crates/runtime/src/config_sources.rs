use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{ConfigDocument, ConfigEntry, ConfigError, ConfigSource};
use crate::json::JsonValue;

pub(crate) fn default_config_home() -> PathBuf {
    std::env::var_os("CLAW_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claw")))
        .unwrap_or_else(|| PathBuf::from(".claw"))
}

pub(crate) fn discovered_entries(cwd: &Path, config_home: &Path) -> Vec<ConfigEntry> {
    let user_legacy_path = config_home.parent().map_or_else(
        || PathBuf::from(".claw.json"),
        |parent| parent.join(".claw.json"),
    );
    vec![
        ConfigEntry {
            source: ConfigSource::User,
            path: user_legacy_path,
        },
        ConfigEntry {
            source: ConfigSource::User,
            path: config_home.join("settings.json"),
        },
        ConfigEntry {
            source: ConfigSource::Project,
            path: cwd.join(".claw.json"),
        },
        ConfigEntry {
            source: ConfigSource::Project,
            path: cwd.join(".claw").join("settings.json"),
        },
        ConfigEntry {
            source: ConfigSource::Local,
            path: cwd.join(".claw").join("settings.local.json"),
        },
    ]
}

pub(crate) fn writable_path(cwd: &Path, config_home: &Path, source: ConfigSource) -> PathBuf {
    match source {
        ConfigSource::User => config_home.join("settings.json"),
        ConfigSource::Project => cwd.join(".claw").join("settings.json"),
        ConfigSource::Local => cwd.join(".claw").join("settings.local.json"),
    }
}

pub(crate) fn load_documents(
    entries: Vec<ConfigEntry>,
) -> Result<Vec<ConfigDocument>, ConfigError> {
    entries
        .into_iter()
        .map(|entry| {
            let exists = entry.path.exists();
            let document = read_optional_json_object(&entry.path)?;
            Ok(ConfigDocument {
                source: entry.source,
                path: entry.path,
                exists,
                loaded: document.is_some(),
                document,
            })
        })
        .collect()
}

pub(crate) fn read_optional_json_object(
    path: &Path,
) -> Result<Option<BTreeMap<String, JsonValue>>, ConfigError> {
    let is_legacy_config = path.file_name().and_then(|name| name.to_str()) == Some(".claw.json");
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ConfigError::Io(error)),
    };

    if contents.trim().is_empty() {
        return Ok(Some(BTreeMap::new()));
    }

    let parsed = match JsonValue::parse(&contents) {
        Ok(parsed) => parsed,
        Err(_error) if is_legacy_config => return Ok(None),
        Err(error) => return Err(ConfigError::Parse(format!("{}: {error}", path.display()))),
    };
    let Some(object) = parsed.as_object() else {
        if is_legacy_config {
            return Ok(None);
        }
        return Err(ConfigError::Parse(format!(
            "{}: top-level settings value must be a JSON object",
            path.display()
        )));
    };
    Ok(Some(object.clone()))
}
