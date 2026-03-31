use std::{
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
};

use octopus_core::{AppError, PreferencesPort, ShellPreferences};

#[derive(Debug, Clone)]
pub struct PreferencesService {
  path: PathBuf,
  defaults: ShellPreferences,
}

impl PreferencesService {
  pub fn new(path: PathBuf, defaults: ShellPreferences) -> Self {
    Self { path, defaults }
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  fn ensure_parent_dir(&self) -> Result<(), AppError> {
    if let Some(parent) = self.path.parent() {
      fs::create_dir_all(parent)?;
    }

    Ok(())
  }
}

impl PreferencesPort for PreferencesService {
  fn load_preferences(&self) -> Result<ShellPreferences, AppError> {
    match fs::read_to_string(&self.path) {
      Ok(raw) => Ok(serde_json::from_str(&raw)?),
      Err(error) if error.kind() == ErrorKind::NotFound => Ok(self.defaults.clone()),
      Err(error) => Err(error.into()),
    }
  }

  fn save_preferences(&self, preferences: &ShellPreferences) -> Result<ShellPreferences, AppError> {
    self.ensure_parent_dir()?;
    fs::write(&self.path, serde_json::to_vec_pretty(preferences)?)?;
    Ok(preferences.clone())
  }
}
