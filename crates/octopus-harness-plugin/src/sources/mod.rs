use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use harness_contracts::TrustLevel;
use ring::digest;
use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

use crate::{
    DiscoverySource, ManifestLoaderError, ManifestOrigin, ManifestRecord,
    ManifestValidationFailure, PluginManifest, PluginManifestLoader,
};

#[derive(Debug, Default, Clone)]
pub struct FileManifestLoader;

#[async_trait]
impl PluginManifestLoader for FileManifestLoader {
    async fn enumerate(
        &self,
        source: &DiscoverySource,
    ) -> Result<Vec<ManifestRecord>, ManifestLoaderError> {
        let Some((root, expected_trust)) = source_root(source) else {
            return Ok(Vec::new());
        };

        let plugin_root = plugin_root(source, root);
        if !plugin_root.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&plugin_root)
            .map_err(|error| ManifestLoaderError::Io(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| ManifestLoaderError::Io(error.to_string()))?;
        entries.sort_by_key(|entry| entry.path());

        let mut records = Vec::new();
        for entry in entries {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(manifest_path) = manifest_path(&path) else {
                continue;
            };
            records.push(read_manifest(&manifest_path, expected_trust)?);
        }

        Ok(records)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InlineManifestLoader {
    records: Vec<ManifestRecord>,
}

impl InlineManifestLoader {
    pub fn new(records: Vec<ManifestRecord>) -> Self {
        Self { records }
    }
}

#[async_trait]
impl PluginManifestLoader for InlineManifestLoader {
    async fn enumerate(
        &self,
        source: &DiscoverySource,
    ) -> Result<Vec<ManifestRecord>, ManifestLoaderError> {
        if matches!(source, DiscoverySource::Inline) {
            Ok(self.records.clone())
        } else {
            Ok(Vec::new())
        }
    }
}

fn source_root(source: &DiscoverySource) -> Option<(&Path, TrustLevel)> {
    match source {
        DiscoverySource::Workspace(path) => Some((path.as_path(), TrustLevel::AdminTrusted)),
        DiscoverySource::User(path) | DiscoverySource::Project(path) => {
            Some((path.as_path(), TrustLevel::UserControlled))
        }
        DiscoverySource::CargoExtension | DiscoverySource::Inline => None,
    }
}

fn plugin_root(source: &DiscoverySource, root: &Path) -> PathBuf {
    match source {
        DiscoverySource::Workspace(_) => root.join("data/plugins"),
        DiscoverySource::User(_) | DiscoverySource::Project(_) => root.join(".octopus/plugins"),
        DiscoverySource::CargoExtension | DiscoverySource::Inline => root.to_path_buf(),
    }
}

fn manifest_path(plugin_dir: &Path) -> Option<PathBuf> {
    ["plugin.json", "plugin.yaml", "plugin.yml"]
        .into_iter()
        .map(|name| plugin_dir.join(name))
        .find(|path| path.is_file())
}

fn read_manifest(
    path: &Path,
    expected_trust: TrustLevel,
) -> Result<ManifestRecord, ManifestLoaderError> {
    let bytes = fs::read(path).map_err(|error| ManifestLoaderError::Io(error.to_string()))?;
    let origin = ManifestOrigin::File {
        path: path.to_path_buf(),
    };
    let manifest = parse_manifest(path, &bytes, origin.clone())?;

    if manifest.trust_level != expected_trust {
        return Err(validation_error(
            Some(origin),
            Some(manifest.name.to_string()),
            format!(
                "manifest trust_level {:?} does not match source trust {:?}",
                manifest.trust_level, expected_trust
            ),
        ));
    }

    let manifest_hash = digest::digest(&digest::SHA256, &bytes);
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(manifest_hash.as_ref());

    ManifestRecord::new(manifest, origin, hash).map_err(|error| {
        validation_error(
            None,
            None,
            format!("manifest basic validation failed: {error}"),
        )
    })
}

fn parse_manifest(
    path: &Path,
    bytes: &[u8],
    origin: ManifestOrigin,
) -> Result<PluginManifest, ManifestLoaderError> {
    let extension = path.extension().and_then(std::ffi::OsStr::to_str);
    match extension {
        Some("json") => serde_json::from_slice(bytes).map_err(|error| {
            validation_error(Some(origin), None, format!("json parse failed: {error}"))
        }),
        Some("yaml" | "yml") => {
            let text = std::str::from_utf8(bytes).map_err(|error| {
                validation_error(
                    Some(origin.clone()),
                    None,
                    format!("yaml utf8 failed: {error}"),
                )
            })?;
            let docs = YamlLoader::load_from_str(text).map_err(|error| {
                validation_error(
                    Some(origin.clone()),
                    None,
                    format!("yaml parse failed: {error}"),
                )
            })?;
            let Some(document) = docs.first() else {
                return Err(validation_error(
                    Some(origin),
                    None,
                    "yaml document is empty".to_owned(),
                ));
            };
            let value = yaml_to_json(document).map_err(|details| {
                validation_error(
                    Some(origin.clone()),
                    None,
                    format!("yaml convert failed: {details}"),
                )
            })?;
            serde_json::from_value(value).map_err(|error| {
                validation_error(
                    Some(origin),
                    None,
                    format!("yaml manifest decode failed: {error}"),
                )
            })
        }
        _ => Err(validation_error(
            Some(origin),
            None,
            "unsupported manifest extension".to_owned(),
        )),
    }
}

fn yaml_to_json(yaml: &Yaml) -> Result<Value, String> {
    match yaml {
        Yaml::Real(value) => value
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| format!("invalid real value: {value}")),
        Yaml::Integer(value) => Ok(Value::Number(Number::from(*value))),
        Yaml::String(value) => Ok(Value::String(value.clone())),
        Yaml::Boolean(value) => Ok(Value::Bool(*value)),
        Yaml::Array(values) => values
            .iter()
            .map(yaml_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Yaml::Hash(hash) => {
            let mut object = Map::new();
            for (key, value) in hash {
                let Yaml::String(key) = key else {
                    return Err("yaml object keys must be strings".to_owned());
                };
                object.insert(key.clone(), yaml_to_json(value)?);
            }
            Ok(Value::Object(object))
        }
        Yaml::Null => Ok(Value::Null),
        Yaml::BadValue | Yaml::Alias(_) => Err("unsupported yaml value".to_owned()),
    }
}

fn validation_error(
    origin: Option<ManifestOrigin>,
    partial_name: Option<String>,
    details: String,
) -> ManifestLoaderError {
    ManifestLoaderError::Validation(ManifestValidationFailure {
        origin,
        partial_name,
        details,
    })
}
