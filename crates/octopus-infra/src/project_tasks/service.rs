use super::*;

#[async_trait]
impl ProjectTaskService for InfraWorkspaceService {
    async fn list_tasks(&self, project_id: &str) -> Result<Vec<ProjectTaskRecord>, AppError> {
        self.ensure_project_exists(project_id)?;
        let mut records = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_task_records(&mut records);
        Ok(records)
    }

    async fn create_task(
        &self,
        project_id: &str,
        user_id: &str,
        request: CreateTaskRequest,
    ) -> Result<ProjectTaskRecord, AppError> {
        self.ensure_project_exists(project_id)?;
        let workspace_id = self.state.workspace_id()?;
        let now = timestamp_now();
        let record = ProjectTaskRecord {
            id: format!("task-{}", Uuid::new_v4()),
            workspace_id,
            project_id: project_id.to_string(),
            title: normalize_required_task_text(&request.title, "task title")?,
            goal: normalize_required_task_text(&request.goal, "task goal")?,
            brief: normalize_required_task_text(&request.brief, "task brief")?,
            default_actor_ref: normalize_required_task_text(
                &request.default_actor_ref,
                "default actor",
            )?,
            status: "ready".into(),
            schedule_spec: normalize_optional_task_text(request.schedule_spec),
            next_run_at: None,
            last_run_at: None,
            active_task_run_id: None,
            latest_result_summary: None,
            latest_failure_category: None,
            latest_transition: None,
            view_status: default_task_view_status(),
            attention_reasons: Vec::new(),
            attention_updated_at: None,
            analytics_summary: TaskAnalyticsSummary::default(),
            context_bundle: normalize_task_context_bundle(request.context_bundle),
            latest_deliverable_refs: Vec::new(),
            latest_artifact_refs: Vec::new(),
            created_by: user_id.to_string(),
            updated_by: None,
            created_at: now,
            updated_at: now,
        };

        persist_project_task_record(&self.state.open_db()?, &record, false)?;
        let mut tasks = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
        tasks.push(record.clone());
        Ok(record)
    }

    async fn get_task(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<ProjectTaskRecord, AppError> {
        self.state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?
            .iter()
            .find(|record| record.project_id == project_id && record.id == task_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("task not found"))
    }

    async fn update_task(
        &self,
        project_id: &str,
        task_id: &str,
        user_id: &str,
        request: UpdateTaskRequest,
    ) -> Result<ProjectTaskRecord, AppError> {
        let existing = self.get_task(project_id, task_id).await?;
        let updated = ProjectTaskRecord {
            title: match request.title {
                Some(value) => normalize_required_task_text(&value, "task title")?,
                None => existing.title.clone(),
            },
            goal: match request.goal {
                Some(value) => normalize_required_task_text(&value, "task goal")?,
                None => existing.goal.clone(),
            },
            brief: match request.brief {
                Some(value) => normalize_required_task_text(&value, "task brief")?,
                None => existing.brief.clone(),
            },
            default_actor_ref: match request.default_actor_ref {
                Some(value) => normalize_required_task_text(&value, "default actor")?,
                None => existing.default_actor_ref.clone(),
            },
            schedule_spec: request
                .schedule_spec
                .map(Some)
                .map(normalize_optional_task_text)
                .unwrap_or(existing.schedule_spec.clone()),
            context_bundle: request
                .context_bundle
                .map(normalize_task_context_bundle)
                .unwrap_or_else(|| existing.context_bundle.clone()),
            updated_by: Some(user_id.to_string()),
            updated_at: timestamp_now(),
            ..existing
        };

        persist_project_task_record(&self.state.open_db()?, &updated, true)?;
        let mut tasks = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
        Self::replace_or_push(&mut tasks, updated.clone(), |record| record.id == task_id);
        Ok(updated)
    }

    async fn save_task(&self, record: ProjectTaskRecord) -> Result<ProjectTaskRecord, AppError> {
        self.ensure_project_exists(&record.project_id)?;
        persist_project_task_record(&self.state.open_db()?, &record, true)?;
        let mut tasks = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
        Self::replace_or_push(&mut tasks, record.clone(), |item| item.id == record.id);
        Ok(record)
    }

    async fn list_task_runs(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<ProjectTaskRunRecord>, AppError> {
        let mut records = self
            .state
            .project_task_runs
            .lock()
            .map_err(|_| AppError::runtime("project task runs mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id && record.task_id == task_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_task_run_records(&mut records);
        Ok(records)
    }

    async fn save_task_run(
        &self,
        record: ProjectTaskRunRecord,
    ) -> Result<ProjectTaskRunRecord, AppError> {
        self.ensure_project_exists(&record.project_id)?;
        persist_project_task_run_record(&self.state.open_db()?, &record, true)?;
        let mut runs = self
            .state
            .project_task_runs
            .lock()
            .map_err(|_| AppError::runtime("project task runs mutex poisoned"))?;
        Self::replace_or_push(&mut runs, record.clone(), |item| item.id == record.id);
        Ok(record)
    }

    async fn list_task_interventions(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<ProjectTaskInterventionRecord>, AppError> {
        let mut records = self
            .state
            .project_task_interventions
            .lock()
            .map_err(|_| AppError::runtime("project task interventions mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id && record.task_id == task_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_task_intervention_records(&mut records);
        Ok(records)
    }

    async fn create_task_intervention(
        &self,
        project_id: &str,
        task_id: &str,
        user_id: &str,
        request: CreateTaskInterventionRequest,
    ) -> Result<ProjectTaskInterventionRecord, AppError> {
        let task = self.get_task(project_id, task_id).await?;
        let task_run_id = normalize_optional_task_text(request.task_run_id);
        let intervention_type = normalize_required_task_text(&request.r#type, "intervention type")?;
        let created_at = timestamp_now();
        let payload = request.payload;
        let target_run_id = task_run_id
            .clone()
            .or_else(|| task.active_task_run_id.clone());
        let (updated_run, updated_task, applied_to_session_id, intervention_status) =
            if task_intervention_applies_run_state(&intervention_type) {
                let target_run_id = target_run_id.clone().ok_or_else(|| {
                    AppError::invalid_input(
                        "task intervention requires a target task run or active task run",
                    )
                })?;
                let target_run = self
                    .list_task_runs(project_id, task_id)
                    .await?
                    .into_iter()
                    .find(|record| record.id == target_run_id)
                    .ok_or_else(|| AppError::not_found("task run not found"))?;
                let next_run =
                    apply_task_intervention_to_run(&target_run, &intervention_type, created_at)
                        .ok_or_else(|| {
                            AppError::invalid_input("unsupported task intervention type")
                        })?;
                let next_task = apply_task_intervention_to_task(
                    &task,
                    &next_run,
                    &intervention_type,
                    user_id,
                    created_at,
                );
                (
                    Some(next_run.clone()),
                    next_task,
                    next_run.session_id.clone(),
                    "applied".to_string(),
                )
            } else {
                let task_runs = self.list_task_runs(project_id, task_id).await?;
                let accepted_run = target_run_id.as_ref().and_then(|run_id| {
                    task_runs
                        .iter()
                        .find(|record| &record.id == run_id)
                        .and_then(|record| {
                            apply_accepted_task_intervention_to_run(
                                record,
                                &intervention_type,
                                &payload,
                            )
                        })
                });
                let accepted_task = apply_accepted_task_intervention_to_task(
                    &task,
                    target_run_id.clone(),
                    &intervention_type,
                    &payload,
                    user_id,
                    created_at,
                );
                (accepted_run, accepted_task, None, "accepted".to_string())
            };
        let record = ProjectTaskInterventionRecord {
            id: format!("task-intervention-{}", Uuid::new_v4()),
            workspace_id: task.workspace_id,
            project_id: project_id.to_string(),
            task_id: task_id.to_string(),
            task_run_id,
            r#type: intervention_type,
            payload,
            created_by: user_id.to_string(),
            created_at,
            applied_to_session_id,
            status: intervention_status,
        };
        let mut connection = self.state.open_db()?;
        let transaction = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        persist_project_task_intervention_record(&transaction, &record, false)?;
        if let Some(run) = updated_run.as_ref() {
            persist_project_task_run_record(&transaction, run, true)?;
        }
        if let Some(task) = updated_task.as_ref() {
            persist_project_task_record(&transaction, task, true)?;
        }
        transaction
            .commit()
            .map_err(|error| AppError::database(error.to_string()))?;
        let mut interventions = self
            .state
            .project_task_interventions
            .lock()
            .map_err(|_| AppError::runtime("project task interventions mutex poisoned"))?;
        interventions.push(record.clone());
        drop(interventions);
        if let Some(run) = updated_run {
            let run_id = run.id.clone();
            let mut runs = self
                .state
                .project_task_runs
                .lock()
                .map_err(|_| AppError::runtime("project task runs mutex poisoned"))?;
            Self::replace_or_push(&mut runs, run, |item| item.id == run_id);
        }
        if let Some(task) = updated_task {
            let mut tasks = self
                .state
                .project_tasks
                .lock()
                .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
            Self::replace_or_push(&mut tasks, task, |item| item.id == task_id);
        }
        Ok(record)
    }
}
