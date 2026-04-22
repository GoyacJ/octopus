use super::*;

#[path = "artifacts_inbox_knowledge/artifact_service.rs"]
mod artifact_service;
#[path = "artifacts_inbox_knowledge/inbox_service.rs"]
mod inbox_service;
#[path = "artifacts_inbox_knowledge/knowledge_service.rs"]
mod knowledge_service;
#[path = "artifacts_inbox_knowledge/observation_service.rs"]
mod observation_service;

#[cfg(test)]
mod tests;
