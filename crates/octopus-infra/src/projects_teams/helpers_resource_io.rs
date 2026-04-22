use super::*;

impl InfraWorkspaceService {
    pub(crate) fn resource_content_type(name: &str, location: Option<&str>) -> Option<String> {
        let extension = Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .or_else(|| {
                location.and_then(|value| {
                    Path::new(value)
                        .extension()
                        .and_then(|extension| extension.to_str())
                })
            })?
            .to_ascii_lowercase();

        let content_type = match extension.as_str() {
            "md" => "text/markdown",
            "txt" | "csv" | "rs" | "ts" | "tsx" | "js" | "jsx" | "vue" | "py" | "go" | "java"
            | "kt" | "swift" | "c" | "cc" | "cpp" | "h" | "hpp" | "html" | "css" | "yaml"
            | "yml" | "toml" | "sql" | "sh" => "text/plain",
            "json" => "application/json",
            "pdf" => "application/pdf",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "m4a" => "audio/mp4",
            "mp4" => "video/mp4",
            "mov" => "video/quicktime",
            "webm" => "video/webm",
            _ => "application/octet-stream",
        };

        Some(content_type.into())
    }

    pub(crate) fn resource_preview_kind(
        kind: &str,
        name: &str,
        location: Option<&str>,
        content_type: Option<&str>,
    ) -> String {
        if kind == "folder" {
            return "folder".into();
        }
        if kind == "url" {
            return "url".into();
        }

        let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
        if content_type == "text/markdown" {
            return "markdown".into();
        }
        if content_type.starts_with("image/") {
            return "image".into();
        }
        if content_type == "application/pdf" {
            return "pdf".into();
        }
        if content_type.starts_with("audio/") {
            return "audio".into();
        }
        if content_type.starts_with("video/") {
            return "video".into();
        }
        if content_type.starts_with("text/") || content_type == "application/json" {
            let extension = Path::new(name)
                .extension()
                .and_then(|extension| extension.to_str())
                .or_else(|| {
                    location.and_then(|value| {
                        Path::new(value)
                            .extension()
                            .and_then(|extension| extension.to_str())
                    })
                })
                .map(|extension| extension.to_ascii_lowercase());
            if extension.as_deref() == Some("md") {
                return "markdown".into();
            }
            if matches!(
                extension.as_deref(),
                Some(
                    "rs" | "ts"
                        | "tsx"
                        | "js"
                        | "jsx"
                        | "vue"
                        | "py"
                        | "go"
                        | "java"
                        | "kt"
                        | "swift"
                        | "c"
                        | "cc"
                        | "cpp"
                        | "h"
                        | "hpp"
                        | "html"
                        | "css"
                        | "json"
                        | "yaml"
                        | "yml"
                        | "toml"
                        | "sql"
                        | "sh"
                )
            ) {
                return "code".into();
            }
            return "text".into();
        }

        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".md") {
            return "markdown".into();
        }
        if lower.ends_with(".pdf") {
            return "pdf".into();
        }
        if matches!(
            lower.rsplit('.').next(),
            Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg")
        ) {
            return "image".into();
        }
        if matches!(
            lower.rsplit('.').next(),
            Some("mp3" | "wav" | "ogg" | "m4a")
        ) {
            return "audio".into();
        }
        if matches!(lower.rsplit('.').next(), Some("mp4" | "mov" | "webm")) {
            return "video".into();
        }
        if matches!(
            lower.rsplit('.').next(),
            Some(
                "rs" | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "vue"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "swift"
                    | "c"
                    | "cc"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "html"
                    | "css"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "sql"
                    | "sh"
            )
        ) {
            return "code".into();
        }

        "binary".into()
    }

    pub(crate) fn build_metadata_resource_record(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let kind = input.kind.trim().to_string();
        let name = Self::normalize_resource_name(&input.name)?;
        let location = Self::normalize_resource_location(input.location);
        let scope = self.normalize_resource_scope(
            input.project_id.as_deref(),
            input.scope.as_deref().unwrap_or_default(),
        )?;
        let visibility =
            self.normalize_resource_visibility(input.visibility.as_deref().unwrap_or("public"))?;
        let content_type = if kind == "url" {
            None
        } else {
            Self::resource_content_type(&name, location.as_deref())
        };
        let preview_kind =
            Self::resource_preview_kind(&kind, &name, location.as_deref(), content_type.as_deref());

        Ok(WorkspaceResourceRecord {
            id: format!("res-{}", Uuid::new_v4()),
            workspace_id: workspace_id.into(),
            project_id: input.project_id,
            kind: kind.clone(),
            name,
            location,
            origin: if kind == "url" {
                "generated".into()
            } else {
                "source".into()
            },
            scope,
            visibility,
            owner_user_id: owner_user_id.into(),
            storage_path: None,
            content_type,
            byte_size: None,
            preview_kind,
            status: "healthy".into(),
            updated_at: timestamp_now(),
            tags: input.tags,
            source_artifact_id: input.source_artifact_id,
        })
    }

    pub(crate) fn ensure_import_has_files(
        &self,
        files: &[WorkspaceResourceFolderUploadEntry],
    ) -> Result<(), AppError> {
        if files.is_empty() {
            Err(AppError::invalid_input(
                "resource import requires at least one file",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn infer_folder_root_name(
        &self,
        files: &[WorkspaceResourceFolderUploadEntry],
    ) -> Option<String> {
        let mut names = files
            .iter()
            .filter_map(|entry| entry.relative_path.split('/').next())
            .map(str::trim)
            .filter(|value: &&str| !value.is_empty())
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        if names.len() == 1 {
            names.into_iter().next()
        } else {
            None
        }
    }

    pub(crate) fn trim_folder_root_prefix(
        &self,
        root_dir_name: Option<&str>,
        files: Vec<WorkspaceResourceFolderUploadEntry>,
    ) -> Result<Vec<WorkspaceResourceFolderUploadEntry>, AppError> {
        let Some(root_dir_name) = root_dir_name.filter(|value: &&str| !value.trim().is_empty())
        else {
            return Ok(files);
        };

        files
            .into_iter()
            .map(|entry| {
                let relative_path = entry.relative_path.replace('\\', "/");
                let trimmed = relative_path
                    .strip_prefix(&format!("{root_dir_name}/"))
                    .unwrap_or(&relative_path)
                    .to_string();
                Ok(WorkspaceResourceFolderUploadEntry {
                    relative_path: trimmed,
                    ..entry
                })
            })
            .collect()
    }

    pub(crate) fn normalize_uploaded_relative_path(&self, raw: &str) -> Result<PathBuf, AppError> {
        let normalized = raw.trim().replace('\\', "/");
        if normalized.is_empty() {
            return Err(AppError::invalid_input("resource file path is required"));
        }
        let path = Path::new(&normalized);
        if path.is_absolute() {
            return Err(AppError::invalid_input(
                "resource file path must be relative",
            ));
        }

        let mut safe = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => safe.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(AppError::invalid_input("resource file path is invalid"));
                }
            }
        }

        if safe.as_os_str().is_empty() {
            return Err(AppError::invalid_input("resource file path is invalid"));
        }
        Ok(safe)
    }

    pub(crate) fn leaf_name(raw: &str) -> Result<String, AppError> {
        let normalized = raw.trim();
        let file_name = Path::new(normalized)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::invalid_input("resource name is invalid"))?;
        Ok(file_name.to_string())
    }

    pub(crate) fn unique_target_path(&self, candidate: PathBuf, is_dir: bool) -> PathBuf {
        if !candidate.exists() {
            return candidate;
        }

        let suffix = &Uuid::new_v4().simple().to_string()[..8];
        if is_dir {
            let file_name = candidate
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("resource");
            return candidate.with_file_name(format!("{file_name}-{suffix}"));
        }

        let stem = candidate
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("resource");
        match candidate
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some(extension) => candidate.with_file_name(format!("{stem}-{suffix}.{extension}")),
            None => candidate.with_file_name(format!("{stem}-{suffix}")),
        }
    }

    pub(crate) fn write_single_imported_file(
        &self,
        target_path: &Path,
        entry: &WorkspaceResourceFolderUploadEntry,
    ) -> Result<(), AppError> {
        let bytes = BASE64_STANDARD
            .decode(entry.data_base64.as_bytes())
            .map_err(|error| AppError::invalid_input(error.to_string()))?;
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
        Ok(())
    }

    pub(crate) fn write_imported_resource(
        &self,
        workspace_id: &str,
        project_id: Option<&str>,
        owner_user_id: &str,
        scope: String,
        visibility: String,
        input: WorkspaceResourceImportInput,
        target_root: &Path,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let name = Self::normalize_resource_name(&input.name)?;
        let files = self.trim_folder_root_prefix(input.root_dir_name.as_deref(), input.files)?;
        let is_folder = input
            .root_dir_name
            .as_deref()
            .is_some_and(|value: &str| !value.trim().is_empty())
            || files.len() > 1
            || files
                .iter()
                .any(|entry| entry.relative_path.replace('\\', "/").contains('/'));

        if is_folder {
            let folder_name = Self::leaf_name(input.root_dir_name.as_deref().unwrap_or(&name))?;
            let absolute_root = self.unique_target_path(target_root.join(folder_name), true);
            fs::create_dir_all(&absolute_root)?;
            for entry in &files {
                let relative_path = self.normalize_uploaded_relative_path(&entry.relative_path)?;
                self.write_single_imported_file(&absolute_root.join(relative_path), entry)?;
            }
            let storage_path = self.display_storage_path(&absolute_root);
            return Ok(WorkspaceResourceRecord {
                id: format!("res-{}", Uuid::new_v4()),
                workspace_id: workspace_id.into(),
                project_id: project_id.map(str::to_string),
                kind: "folder".into(),
                name,
                location: Some(storage_path.clone()),
                origin: "source".into(),
                scope,
                visibility,
                owner_user_id: owner_user_id.into(),
                storage_path: Some(storage_path),
                content_type: None,
                byte_size: None,
                preview_kind: "folder".into(),
                status: "healthy".into(),
                updated_at: timestamp_now(),
                tags: input.tags.unwrap_or_default(),
                source_artifact_id: None,
            });
        }

        let entry = files
            .into_iter()
            .next()
            .ok_or_else(|| AppError::invalid_input("resource import requires at least one file"))?;
        let file_name = Self::leaf_name(&entry.file_name)?;
        let absolute_path = self.unique_target_path(target_root.join(&file_name), false);
        self.write_single_imported_file(&absolute_path, &entry)?;
        let storage_path = self.display_storage_path(&absolute_path);
        let content_type = if entry.content_type.trim().is_empty() {
            Self::resource_content_type(&file_name, Some(&storage_path))
        } else {
            Some(entry.content_type.trim().into())
        };
        Ok(WorkspaceResourceRecord {
            id: format!("res-{}", Uuid::new_v4()),
            workspace_id: workspace_id.into(),
            project_id: project_id.map(str::to_string),
            kind: "file".into(),
            name,
            location: Some(storage_path.clone()),
            origin: "source".into(),
            scope,
            visibility,
            owner_user_id: owner_user_id.into(),
            storage_path: Some(storage_path.clone()),
            content_type: content_type.clone(),
            byte_size: Some(entry.byte_size),
            preview_kind: Self::resource_preview_kind(
                "file",
                &file_name,
                Some(&storage_path),
                content_type.as_deref(),
            ),
            status: "healthy".into(),
            updated_at: timestamp_now(),
            tags: input.tags.unwrap_or_default(),
            source_artifact_id: None,
        })
    }

    pub(crate) fn collect_resource_children(
        root: &Path,
        current: &Path,
        children: &mut Vec<WorkspaceResourceChildrenRecord>,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                Self::collect_resource_children(root, &path, children)?;
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let relative_path = path
                .strip_prefix(root)
                .map_err(|_| AppError::runtime("resource child path is invalid"))?
                .to_string_lossy()
                .replace('\\', "/");
            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = fs::metadata(&path)?;
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or_else(timestamp_now);
            let content_type = Self::resource_content_type(&file_name, Some(&relative_path));
            children.push(WorkspaceResourceChildrenRecord {
                name: file_name.clone(),
                relative_path,
                kind: "file".into(),
                preview_kind: Self::resource_preview_kind(
                    "file",
                    &file_name,
                    Some(&file_name),
                    content_type.as_deref(),
                ),
                content_type,
                byte_size: Some(metadata.len()),
                updated_at: modified_at,
            });
        }
        Ok(())
    }

    pub(crate) fn delete_managed_resource_storage(
        &self,
        record: &WorkspaceResourceRecord,
    ) -> Result<(), AppError> {
        let Some(storage_path) = record.storage_path.as_deref() else {
            return Ok(());
        };
        let absolute_path = self.resolve_storage_path(storage_path);
        if !absolute_path.exists() {
            return Ok(());
        }
        if absolute_path.is_dir() {
            fs::remove_dir_all(absolute_path)?;
        } else {
            fs::remove_file(absolute_path)?;
        }
        Ok(())
    }

    pub(crate) fn apply_resource_update(
        &self,
        record: &mut WorkspaceResourceRecord,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<(), AppError> {
        if let Some(name) = input.name {
            record.name = Self::normalize_resource_name(&name)?;
        }
        if input.location.is_some() {
            record.location = Self::normalize_resource_location(input.location);
        }
        if let Some(visibility) = input.visibility {
            record.visibility = self.normalize_resource_visibility(&visibility)?;
        }
        if let Some(status) = input.status {
            record.status = Self::normalize_resource_status(&status)?;
        }
        if let Some(tags) = input.tags {
            record.tags = tags;
        }
        record.updated_at = timestamp_now();
        Ok(())
    }
}
