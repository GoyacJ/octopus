use super::*;

fn personal_pet_defaults(
    workspace_id: &str,
    owner_user_id: &str,
) -> (String, PetAgentExtensionRecord) {
    let mut hasher = Sha256::new();
    hasher.update(workspace_id.as_bytes());
    hasher.update(b":");
    hasher.update(owner_user_id.as_bytes());
    let digest = hasher.finalize();
    let species =
        PERSONAL_PET_SPECIES_REGISTRY[(digest[0] as usize) % PERSONAL_PET_SPECIES_REGISTRY.len()];
    let pet_id = format!("pet-{owner_user_id}");
    let display_name = format!("{}伙伴", species);
    let summary = format!("{display_name} 会陪着主人一起完成日常工作。");
    let greeting = format!("嗨，我是 {display_name}，今天一起推进事情吧。");
    let favorite_snack = match species {
        "duck" | "goose" => "玉米粒",
        "cat" | "dragon" | "octopus" => "新鲜小虾",
        "owl" | "ghost" => "夜宵",
        "penguin" | "turtle" | "snail" => "海藻沙拉",
        "axolotl" | "capybara" => "蔬果拼盘",
        "cactus" | "robot" => "阳光和电量",
        "rabbit" | "mushroom" | "chonk" | "blob" => "胡萝卜饼干",
        _ => "零食",
    };
    let extension = PetAgentExtensionRecord {
        pet_id: pet_id.clone(),
        workspace_id: workspace_id.into(),
        owner_user_id: owner_user_id.into(),
        species: species.into(),
        display_name,
        avatar_label: format!("{species} mascot"),
        summary,
        greeting,
        mood: "happy".into(),
        favorite_snack: favorite_snack.into(),
        prompt_hints: vec![
            "帮我整理一下今天的重点".into(),
            "我们接下来先做什么？".into(),
            "给我一句鼓励的话".into(),
        ],
        fallback_asset: species.into(),
        rive_asset: None,
        state_machine: None,
        updated_at: timestamp_now(),
    };
    (pet_id, extension)
}

pub(crate) fn pet_context_key(owner_user_id: &str, project_id: Option<&str>) -> String {
    match project_id {
        Some(project_id) if !project_id.trim().is_empty() => {
            format!("{owner_user_id}::{PET_CONTEXT_SCOPE_PROJECT}::{project_id}")
        }
        _ => format!("{owner_user_id}::{PET_CONTEXT_SCOPE_HOME}"),
    }
}

pub(crate) fn default_pet_profile(
    pet_id: &str,
    owner_user_id: &str,
    extension: &PetAgentExtensionRecord,
) -> PetProfile {
    PetProfile {
        id: pet_id.into(),
        species: extension.species.clone(),
        display_name: extension.display_name.clone(),
        owner_user_id: owner_user_id.into(),
        avatar_label: extension.avatar_label.clone(),
        summary: extension.summary.clone(),
        greeting: extension.greeting.clone(),
        mood: extension.mood.clone(),
        favorite_snack: extension.favorite_snack.clone(),
        prompt_hints: extension.prompt_hints.clone(),
        fallback_asset: extension.fallback_asset.clone(),
        rive_asset: extension.rive_asset.clone(),
        state_machine: extension.state_machine.clone(),
    }
}

pub(crate) fn default_workspace_pet_presence_for(pet_id: &str) -> PetPresenceState {
    PetPresenceState {
        pet_id: pet_id.into(),
        is_visible: true,
        chat_open: false,
        motion_state: "idle".into(),
        unread_count: 0,
        last_interaction_at: 0,
        position: PetPosition { x: 0, y: 0 },
    }
}

pub(crate) fn map_pet_message(pet_id: &str, message: &octopus_core::RuntimeMessage) -> PetMessage {
    PetMessage {
        id: message.id.clone(),
        pet_id: pet_id.into(),
        sender: if message.sender_type == "assistant" {
            "pet".into()
        } else {
            "user".into()
        },
        content: message.content.clone(),
        timestamp: message.timestamp,
    }
}

pub(crate) fn load_runtime_messages_for_conversation(
    connection: &Connection,
    conversation_id: &str,
    pet_id: &str,
) -> Result<Vec<PetMessage>, AppError> {
    let detail_json: Option<String> = connection
        .query_row(
            "SELECT detail_json FROM runtime_session_projections WHERE conversation_id = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![conversation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    let Some(detail_json) = detail_json else {
        return Ok(vec![]);
    };
    let detail: octopus_core::RuntimeSessionDetail = serde_json::from_str(&detail_json)?;
    Ok(detail
        .messages
        .iter()
        .map(|message| map_pet_message(pet_id, message))
        .collect())
}

pub(crate) fn row_to_pet_presence(row: &rusqlite::Row<'_>) -> rusqlite::Result<PetPresenceState> {
    Ok(PetPresenceState {
        pet_id: row.get(4)?,
        is_visible: row.get::<_, i64>(5)? != 0,
        chat_open: row.get::<_, i64>(6)? != 0,
        motion_state: row.get(7)?,
        unread_count: row.get::<_, i64>(8)? as u64,
        last_interaction_at: row.get::<_, i64>(9)? as u64,
        position: PetPosition {
            x: row.get(10)?,
            y: row.get(11)?,
        },
    })
}

pub(crate) fn load_pet_presences(
    connection: &Connection,
) -> Result<HashMap<String, PetPresenceState>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT scope_key, owner_user_id, context_scope, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y FROM pet_presence",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row_to_pet_presence(row)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn row_to_pet_binding(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PetConversationBinding> {
    Ok(PetConversationBinding {
        pet_id: row.get(4)?,
        workspace_id: row.get(5)?,
        owner_user_id: row
            .get::<_, Option<String>>(1)?
            .unwrap_or_else(|| BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID.into()),
        context_scope: row
            .get::<_, Option<String>>(2)?
            .unwrap_or_else(|| PET_CONTEXT_SCOPE_HOME.into()),
        project_id: row.get(3)?,
        conversation_id: row.get(6)?,
        session_id: row.get(7)?,
        updated_at: row.get::<_, i64>(8)? as u64,
    })
}

pub(crate) fn load_pet_bindings(
    connection: &Connection,
) -> Result<HashMap<String, PetConversationBinding>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT scope_key, owner_user_id, context_scope, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at FROM pet_conversation_bindings",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row_to_pet_binding(row)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_pet_agent_extensions(
    connection: &Connection,
) -> Result<HashMap<String, PetAgentExtensionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT pet_id, workspace_id, owner_user_id, species, display_name, avatar_label,
                    summary, greeting, mood, favorite_snack, prompt_hints_json, fallback_asset,
                    rive_asset, state_machine, updated_at
             FROM pet_agent_extensions",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let prompt_hints_raw: String = row.get(10)?;
            Ok((
                row.get::<_, String>(2)?,
                PetAgentExtensionRecord {
                    pet_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    owner_user_id: row.get(2)?,
                    species: row.get(3)?,
                    display_name: row.get(4)?,
                    avatar_label: row.get(5)?,
                    summary: row.get(6)?,
                    greeting: row.get(7)?,
                    mood: row.get(8)?,
                    favorite_snack: row.get(9)?,
                    prompt_hints: serde_json::from_str(&prompt_hints_raw).unwrap_or_default(),
                    fallback_asset: row.get(11)?,
                    rive_asset: row.get(12)?,
                    state_machine: row.get(13)?,
                    updated_at: row.get::<_, i64>(14)? as u64,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn ensure_personal_pet_for_user(
    connection: &Connection,
    workspace_id: &str,
    owner_user_id: &str,
) -> Result<(), AppError> {
    let existing_pet_id: Option<String> = connection
        .query_row(
            "SELECT pet_id FROM pet_agent_extensions WHERE workspace_id = ?1 AND owner_user_id = ?2",
            params![workspace_id, owner_user_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if existing_pet_id.is_some() {
        return Ok(());
    }

    let (pet_id, extension) = personal_pet_defaults(workspace_id, owner_user_id);
    let pet_record = AgentRecord {
        id: pet_id.clone(),
        workspace_id: workspace_id.into(),
        project_id: None,
        scope: "personal".into(),
        owner_user_id: Some(owner_user_id.into()),
        asset_role: PERSONAL_PET_ASSET_ROLE.into(),
        name: extension.display_name.clone(),
        avatar_path: None,
        avatar: None,
        personality: extension.summary.clone(),
        tags: vec!["pet".into(), extension.species.clone()],
        prompt: format!(
            "{} 你是 {} 的个人宠物伙伴，保持亲切、轻量、鼓励式的交流。",
            extension.greeting, owner_user_id
        ),
        builtin_tool_keys: Vec::new(),
        skill_ids: Vec::new(),
        mcp_server_names: Vec::new(),
        task_domains: normalize_task_domains(Vec::new()),
        manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
        default_model_strategy: default_model_strategy(),
        capability_policy: capability_policy_from_sources(&[], &[], &[]),
        permission_envelope: default_permission_envelope(),
        memory_policy: default_agent_memory_policy(),
        delegation_policy: default_agent_delegation_policy(),
        approval_preference: default_approval_preference(),
        output_contract: default_output_contract(),
        shared_capability_policy: default_agent_shared_capability_policy(),
        integration_source: None,
        trust_metadata: default_asset_trust_metadata(),
        dependency_resolution: Vec::new(),
        import_metadata: default_asset_import_metadata(),
        description: extension.summary.clone(),
        status: "active".into(),
        updated_at: extension.updated_at,
    };
    write_agent_record(connection, &pet_record, false)?;
    connection
        .execute(
            "INSERT INTO pet_agent_extensions (
                pet_id, workspace_id, owner_user_id, species, display_name, avatar_label,
                summary, greeting, mood, favorite_snack, prompt_hints_json, fallback_asset,
                rive_asset, state_machine, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15
            )",
            params![
                extension.pet_id,
                extension.workspace_id,
                extension.owner_user_id,
                extension.species,
                extension.display_name,
                extension.avatar_label,
                extension.summary,
                extension.greeting,
                extension.mood,
                extension.favorite_snack,
                json_string(&extension.prompt_hints)?,
                extension.fallback_asset,
                extension.rive_asset,
                extension.state_machine,
                extension.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}
