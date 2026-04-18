use super::adapter_test_support::*;
use super::*;

#[tokio::test]
async fn submit_turn_does_not_create_deliverable_for_ordinary_assistant_replies() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-persistence",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Persistence",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Draft the deliverable body", None),
        )
        .await
        .expect("run");

    assert!(run.deliverable_refs.is_empty());
    let runtime_artifact_id = run
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime artifact id");
    assert!(adapter
        .get_deliverable_detail(&runtime_artifact_id)
        .expect("deliverable detail query")
        .is_none());

    let session_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(session_detail
        .messages
        .iter()
        .filter(|message| message.sender_type == "assistant")
        .all(|message| message.deliverable_refs.as_ref().is_none_or(Vec::is_empty)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_persists_explicit_deliverable_detail_and_versions_across_reload() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the release notes deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("Release Notes Draft".into()),
                preview_kind: "markdown".into(),
                file_name: Some("release-notes.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some(
                    "# Release Notes Draft\n\n- Runtime deliverables now require explicit output."
                        .into(),
                ),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-persistence",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Persistence",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Draft the deliverable body", None),
        )
        .await
        .expect("run");

    let deliverable_ref = run
        .deliverable_refs
        .first()
        .cloned()
        .expect("generated deliverable ref");
    let artifact_id = deliverable_ref.artifact_id.clone();
    let detail = adapter
        .get_deliverable_detail(&artifact_id)
        .expect("deliverable detail query")
        .expect("deliverable detail");
    assert_eq!(detail.id, artifact_id);
    assert_eq!(detail.project_id, octopus_core::DEFAULT_PROJECT_ID);
    assert_eq!(detail.conversation_id, session.summary.conversation_id);
    assert_eq!(detail.session_id, session.summary.id);
    assert_eq!(detail.run_id, run.id);
    assert_eq!(detail.latest_version, 1);
    assert_eq!(detail.latest_version_ref.version, 1);
    assert_eq!(detail.latest_version_ref.artifact_id, detail.id);
    assert_eq!(detail.latest_version_ref.title, "Release Notes Draft");
    assert_eq!(detail.promotion_state, "not-promoted");

    let versions = adapter
        .list_deliverable_versions(&artifact_id)
        .expect("deliverable versions query");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].artifact_id, artifact_id);
    assert_eq!(versions[0].version, 1);
    assert_eq!(
        versions[0].session_id.as_deref(),
        Some(session.summary.id.as_str())
    );
    assert_eq!(versions[0].run_id.as_deref(), Some(run.id.as_str()));

    let content = adapter
        .get_deliverable_version_content(&artifact_id, 1)
        .expect("deliverable content query")
        .expect("deliverable content");
    assert_eq!(content.artifact_id, artifact_id);
    assert_eq!(content.version, 1);
    assert!(content.editable);
    assert_eq!(content.preview_kind, "markdown");
    assert_eq!(content.file_name.as_deref(), Some("release-notes.md"));
    assert!(content
        .text_content
        .as_deref()
        .is_some_and(|value| value.contains("Runtime deliverables now require explicit output.")));

    let session_detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    assert!(session_detail
        .messages
        .iter()
        .rev()
        .find(|message| message.sender_type == "assistant")
        .and_then(|message| message.deliverable_refs.clone())
        .is_some_and(|refs| refs
            .iter()
            .any(|reference| reference.artifact_id == artifact_id)));

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the release notes deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("Release Notes Draft".into()),
                preview_kind: "markdown".into(),
                file_name: Some("release-notes.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some(
                    "# Release Notes Draft\n\n- Runtime deliverables now require explicit output."
                        .into(),
                ),
                data_base64: None,
            }],
        }),
    );
    let reloaded_detail = reloaded
        .get_deliverable_detail(&artifact_id)
        .expect("reloaded detail query")
        .expect("reloaded detail");
    assert_eq!(reloaded_detail.latest_version, 1);
    assert_eq!(reloaded_detail.latest_version_ref.version, 1);
    let reloaded_versions = reloaded
        .list_deliverable_versions(&artifact_id)
        .expect("reloaded versions query");
    assert_eq!(reloaded_versions.len(), 1);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn creating_new_deliverable_version_preserves_previous_versions() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the first explicit deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("First deliverable".into()),
                preview_kind: "markdown".into(),
                file_name: Some("first-deliverable.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some("# First deliverable\n\nProduce the first draft".into()),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-versioning",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Versioning",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Produce the first draft", None),
        )
        .await
        .expect("run");

    let artifact_id = run
        .deliverable_refs
        .first()
        .map(|reference| reference.artifact_id.clone())
        .expect("generated deliverable artifact id");
    let updated = adapter
        .create_deliverable_version(
            &artifact_id,
            CreateDeliverableVersionInput {
                title: Some("Edited deliverable".into()),
                preview_kind: "markdown".into(),
                text_content: Some("Edited deliverable body".into()),
                data_base64: None,
                content_type: Some("text/markdown".into()),
                source_message_id: Some("msg-edited-version".into()),
                parent_version: Some(1),
            },
        )
        .await
        .expect("create deliverable version");

    assert_eq!(updated.id, artifact_id);
    assert_eq!(updated.latest_version, 2);
    assert_eq!(updated.latest_version_ref.version, 2);
    assert_eq!(updated.title, "Edited deliverable");

    let versions = adapter
        .list_deliverable_versions(&artifact_id)
        .expect("versions query");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[0].parent_version, Some(1));
    assert_eq!(versions[1].version, 1);

    let version_one = adapter
        .get_deliverable_version_content(&artifact_id, 1)
        .expect("version one content query")
        .expect("version one content");
    assert!(version_one
        .text_content
        .as_deref()
        .is_some_and(|value| value.contains("Produce the first draft")));
    let version_two = adapter
        .get_deliverable_version_content(&artifact_id, 2)
        .expect("version two content query")
        .expect("version two content");
    assert_eq!(
        version_two.text_content.as_deref(),
        Some("Edited deliverable body")
    );

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );
    let reloaded_detail = reloaded
        .get_deliverable_detail(&artifact_id)
        .expect("reloaded detail query")
        .expect("reloaded detail");
    assert_eq!(reloaded_detail.latest_version, 2);
    let reloaded_versions = reloaded
        .list_deliverable_versions(&artifact_id)
        .expect("reloaded versions query");
    assert_eq!(reloaded_versions.len(), 2);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn promoting_deliverable_creates_knowledge_record_and_preserves_lineage() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let agent_actor_ref = builtin_agent_actor_ref(&infra).await;
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(ExplicitDeliverableRuntimeModelDriver {
            total_tokens: Some(12),
            content: "Created the reusable guidance deliverable.".into(),
            deliverables: vec![ModelExecutionDeliverable {
                title: Some("Reusable guidance".into()),
                preview_kind: "markdown".into(),
                file_name: Some("reusable-guidance.md".into()),
                content_type: Some("text/markdown".into()),
                text_content: Some(
                    "# Reusable guidance\n\nKeep the finance review tag on approvals.".into(),
                ),
                data_base64: None,
            }],
        }),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-deliverable-promotion",
                octopus_core::DEFAULT_PROJECT_ID,
                "Deliverable Promotion",
                &agent_actor_ref,
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Capture the reusable guidance", None),
        )
        .await
        .expect("run");
    let artifact_id = run
        .deliverable_refs
        .first()
        .map(|reference| reference.artifact_id.clone())
        .expect("generated deliverable artifact id");

    let promoted = adapter
        .promote_deliverable(
            &artifact_id,
            PromoteDeliverableInput {
                title: Some("Reusable guidance".into()),
                summary: Some("Keep the finance review tag on approvals.".into()),
                kind: Some("shared".into()),
            },
        )
        .await
        .expect("promote deliverable");
    assert_eq!(promoted.id, artifact_id);
    assert_eq!(promoted.promotion_state, "promoted");
    let promotion_knowledge_id = promoted
        .promotion_knowledge_id
        .clone()
        .expect("promotion knowledge id");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let knowledge_row: (String, String, String, String, String) = connection
        .query_row(
            "SELECT title, summary, kind, source_type, source_ref
             FROM knowledge_records
             WHERE id = ?1",
            [promotion_knowledge_id.clone()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("promoted knowledge row");
    assert_eq!(knowledge_row.0, "Reusable guidance");
    assert_eq!(knowledge_row.1, "Keep the finance review tag on approvals.");
    assert_eq!(knowledge_row.2, "shared");
    assert_eq!(knowledge_row.3, "artifact");
    assert_eq!(knowledge_row.4, artifact_id);

    let versions = adapter
        .list_deliverable_versions(&artifact_id)
        .expect("deliverable versions query");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].run_id.as_deref(), Some(run.id.as_str()));
    assert_eq!(
        versions[0].session_id.as_deref(),
        Some(session.summary.id.as_str())
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events.iter().any(|event| {
        event.kind.as_deref() == Some("deliverable.promoted")
            && event.target_kind.as_deref() == Some("deliverable")
            && event.target_ref.as_deref() == Some(artifact_id.as_str())
    }));

    let reloaded = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(FixedTokenRuntimeModelDriver {
            total_tokens: Some(12),
        }),
    );
    let reloaded_detail = reloaded
        .get_deliverable_detail(&artifact_id)
        .expect("reloaded detail query")
        .expect("reloaded detail");
    assert_eq!(reloaded_detail.promotion_state, "promoted");
    assert_eq!(
        reloaded_detail.promotion_knowledge_id.as_deref(),
        Some(promotion_knowledge_id.as_str())
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
