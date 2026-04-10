use super::*;

impl RuntimeAdapter {
    pub(super) fn load_configured_model_usage_map(&self) -> Result<HashMap<String, u64>, AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT configured_model_id, used_tokens
                 FROM configured_model_usage_projections",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                let configured_model_id: String = row.get(0)?;
                let used_tokens: i64 = row.get(1)?;
                Ok((configured_model_id, used_tokens))
            })
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut usage = HashMap::new();
        for row in rows {
            let (configured_model_id, used_tokens) =
                row.map_err(|error| AppError::database(error.to_string()))?;
            usage.insert(configured_model_id, used_tokens.max(0) as u64);
        }
        Ok(usage)
    }

    pub(super) fn configured_model_used_tokens(
        &self,
        configured_model_id: &str,
    ) -> Result<u64, AppError> {
        let connection = self.open_db()?;
        let used_tokens = connection
            .query_row(
                "SELECT used_tokens
                 FROM configured_model_usage_projections
                 WHERE configured_model_id = ?1",
                [configured_model_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .unwrap_or(0);
        Ok(used_tokens.max(0) as u64)
    }

    pub(super) fn increment_configured_model_usage(
        &self,
        configured_model_id: &str,
        consumed_tokens: u32,
        updated_at: u64,
    ) -> Result<u64, AppError> {
        let connection = self.open_db()?;
        connection
            .execute(
                "INSERT INTO configured_model_usage_projections
                 (configured_model_id, used_tokens, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(configured_model_id)
                 DO UPDATE SET
                   used_tokens = configured_model_usage_projections.used_tokens + excluded.used_tokens,
                   updated_at = excluded.updated_at",
                params![
                    configured_model_id,
                    i64::from(consumed_tokens),
                    updated_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.configured_model_used_tokens(configured_model_id)
    }

    pub(super) fn ensure_configured_model_quota_available(
        &self,
        configured_model: &ConfiguredModelRecord,
    ) -> Result<(), AppError> {
        let Some(total_tokens) = configured_model
            .token_quota
            .as_ref()
            .and_then(|quota| quota.total_tokens)
        else {
            return Ok(());
        };

        let used_tokens =
            self.configured_model_used_tokens(&configured_model.configured_model_id)?;
        if used_tokens >= total_tokens {
            return Err(AppError::invalid_input(format!(
                "configured model `{}` has reached its total token limit",
                configured_model.configured_model_id
            )));
        }

        Ok(())
    }

    pub(super) fn resolve_consumed_tokens(
        &self,
        configured_model: &ConfiguredModelRecord,
        response: &ExecutionResponse,
    ) -> Result<Option<u32>, AppError> {
        match response.total_tokens {
            Some(total_tokens) => Ok(Some(total_tokens)),
            None if configured_model
                .token_quota
                .as_ref()
                .and_then(|quota| quota.total_tokens)
                .is_some() =>
            {
                Err(AppError::runtime(format!(
                    "configured model `{}` requires provider token usage for quota enforcement",
                    configured_model.configured_model_id
                )))
            }
            None => Ok(None),
        }
    }
}
