use super::*;

#[async_trait]
impl ArtifactService for InfraArtifactService {
    async fn list_artifacts(&self) -> Result<Vec<ArtifactRecord>, AppError> {
        let artifacts = load_artifact_records(&self.state.open_db()?)?;
        *self
            .state
            .artifacts
            .lock()
            .map_err(|_| AppError::runtime("artifacts mutex poisoned"))? = artifacts.clone();
        Ok(artifacts)
    }
}
