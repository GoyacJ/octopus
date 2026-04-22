use super::*;

#[async_trait]
impl KnowledgeService for InfraKnowledgeService {
    async fn list_knowledge(&self) -> Result<Vec<KnowledgeEntryRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .iter()
            .map(|record| KnowledgeEntryRecord {
                id: record.id.clone(),
                workspace_id: record.workspace_id.clone(),
                project_id: record.project_id.clone(),
                title: record.title.clone(),
                scope: record.scope.clone(),
                status: record.status.clone(),
                source_type: record.source_type.clone(),
                source_ref: record.source_ref.clone(),
                updated_at: record.updated_at,
            })
            .collect())
    }
}
