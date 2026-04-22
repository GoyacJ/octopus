use super::*;

pub(crate) fn ensure_resource_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "resources",
        &[
            ("scope", "TEXT"),
            ("visibility", "TEXT"),
            ("owner_user_id", "TEXT"),
            ("storage_path", "TEXT"),
            ("content_type", "TEXT"),
            ("byte_size", "INTEGER"),
            ("preview_kind", "TEXT"),
            ("source_artifact_id", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_knowledge_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "knowledge_records",
        &[
            ("scope", "TEXT"),
            ("visibility", "TEXT"),
            ("owner_user_id", "TEXT"),
        ],
    )?;

    connection
        .execute(
            "UPDATE knowledge_records
             SET scope = CASE
                 WHEN project_id IS NULL THEN 'workspace'
                 ELSE 'project'
             END
             WHERE scope IS NULL OR TRIM(scope) = ''",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "UPDATE knowledge_records
             SET visibility = CASE
                 WHEN COALESCE(scope, '') = 'personal' THEN 'private'
                 ELSE 'public'
             END
             WHERE visibility IS NULL OR TRIM(visibility) = ''",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}
