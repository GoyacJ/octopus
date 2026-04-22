use super::*;

pub(crate) fn validate_create_project_request(
    request: CreateProjectRequest,
) -> Result<CreateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }
    let resource_directory = request.resource_directory.trim();
    if resource_directory.is_empty() {
        return Err(AppError::invalid_input("project resource directory is required").into());
    }
    let leader_agent_id = match request.leader_agent_id {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                return Err(
                    AppError::invalid_input("project leader agent id cannot be empty").into(),
                );
            }
            Some(trimmed)
        }
        None => None,
    };

    Ok(CreateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        resource_directory: resource_directory.into(),
        owner_user_id: request
            .owner_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        member_user_ids: request.member_user_ids.map(|values| {
            values
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        }),
        permission_overrides: request.permission_overrides,
        linked_workspace_assets: None,
        leader_agent_id,
        manager_user_id: request
            .manager_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        preset_code: request
            .preset_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        assignments: None,
    })
}

pub(crate) fn validate_update_project_request(
    request: UpdateProjectRequest,
) -> Result<UpdateProjectRequest, ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("project name is required").into());
    }

    let status = request.status.trim();
    if status != "active" && status != "archived" {
        return Err(AppError::invalid_input("project status must be active or archived").into());
    }
    let resource_directory = request.resource_directory.trim();
    if resource_directory.is_empty() {
        return Err(AppError::invalid_input("project resource directory is required").into());
    }
    let leader_agent_id = match request.leader_agent_id {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                return Err(
                    AppError::invalid_input("project leader agent id cannot be empty").into(),
                );
            }
            Some(trimmed)
        }
        None => None,
    };

    Ok(UpdateProjectRequest {
        name: name.into(),
        description: request.description.trim().into(),
        status: status.into(),
        resource_directory: resource_directory.into(),
        owner_user_id: request
            .owner_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        member_user_ids: request.member_user_ids.map(|values| {
            values
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        }),
        permission_overrides: request.permission_overrides,
        linked_workspace_assets: None,
        leader_agent_id,
        manager_user_id: request
            .manager_user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        preset_code: request
            .preset_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        assignments: None,
    })
}
