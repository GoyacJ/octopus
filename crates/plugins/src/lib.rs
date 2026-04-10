mod discovery;
mod hook_dispatch;
mod hooks;
mod lifecycle;
mod manifest;
#[cfg(test)]
mod split_module_tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use discovery::{
    builtin_plugins, InstallOutcome, InstalledPluginRecord, InstalledPluginRegistry, PluginError,
    PluginInstallSource, PluginKind, PluginLoadFailure, PluginManager, PluginManagerConfig,
    PluginMetadata, PluginRegistry, PluginRegistryReport, PluginSummary, RegisteredPlugin,
    UpdateOutcome,
};
pub use hook_dispatch::PluginTool;
pub use hooks::{HookEvent, HookRunResult, HookRunner};
pub use lifecycle::{BuiltinPlugin, BundledPlugin, ExternalPlugin, Plugin, PluginDefinition};
pub use manifest::{
    load_plugin_from_directory, PluginCommandManifest, PluginHooks, PluginLifecycle,
    PluginManifest, PluginManifestValidationError, PluginPermission, PluginToolDefinition,
    PluginToolManifest, PluginToolPermission,
};

pub(crate) const EXTERNAL_MARKETPLACE: &str = "external";
pub(crate) const BUILTIN_MARKETPLACE: &str = "builtin";
pub(crate) const BUNDLED_MARKETPLACE: &str = "bundled";
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.json";
pub(crate) const REGISTRY_FILE_NAME: &str = "installed.json";
pub(crate) const MANIFEST_FILE_NAME: &str = "plugin.json";
pub(crate) const MANIFEST_RELATIVE_PATH: &str = ".claude-plugin/plugin.json";

#[allow(unused_imports)]
pub(crate) use discovery::*;
#[allow(unused_imports)]
pub(crate) use hook_dispatch::*;
#[allow(unused_imports)]
pub(crate) use lifecycle::*;
#[allow(unused_imports)]
pub(crate) use manifest::*;
