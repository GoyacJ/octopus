use super::*;

pub(super) fn parse_surface_descriptor(value: &Value) -> Result<SurfaceDescriptor, AppError> {
    Ok(SurfaceDescriptor {
        surface: required_string(value, "surface")?,
        protocol_family: required_string(value, "protocolFamily")?,
        transport: value
            .get("transport")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        auth_strategy: value
            .get("authStrategy")
            .and_then(Value::as_str)
            .unwrap_or("bearer")
            .to_string(),
        base_url: value
            .get("baseUrl")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        base_url_policy: value
            .get("baseUrlPolicy")
            .and_then(Value::as_str)
            .unwrap_or("allow_override")
            .to_string(),
        enabled: value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        capabilities: value
            .get("capabilities")
            .and_then(Value::as_array)
            .map(|entries| parse_capabilities(entries))
            .transpose()?
            .unwrap_or_default(),
        runtime_support: RuntimeExecutionSupport::default(),
    })
}

pub(super) fn parse_surface_binding(value: &Value) -> Result<ModelSurfaceBinding, AppError> {
    Ok(ModelSurfaceBinding {
        surface: required_string(value, "surface")?,
        protocol_family: value
            .get("protocolFamily")
            .and_then(Value::as_str)
            .unwrap_or("openai_chat")
            .to_string(),
        enabled: value
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        runtime_support: RuntimeExecutionSupport::default(),
    })
}

pub(super) fn parse_capabilities(entries: &[Value]) -> Result<Vec<CapabilityDescriptor>, AppError> {
    entries
        .iter()
        .map(|entry| {
            if let Some(capability_id) = entry.as_str() {
                return Ok(capability(capability_id));
            }
            Ok(CapabilityDescriptor {
                capability_id: required_string(entry, "capabilityId")?,
                label: entry
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| {
                        entry
                            .get("capabilityId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                    })
                    .to_string(),
            })
        })
        .collect()
}

pub(super) fn required_string(value: &Value, key: &str) -> Result<String, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::invalid_input(format!("registry field `{key}` must be a string")))
}

pub(crate) fn titleize(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn default_credential_env(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "openai" => Some("OPENAI_API_KEY"),
        "xai" => Some("XAI_API_KEY"),
        "deepseek" => Some("DEEPSEEK_API_KEY"),
        "minimax" => Some("MINIMAX_API_KEY"),
        "moonshot" => Some("MOONSHOT_API_KEY"),
        "bigmodel" => Some("BIGMODEL_API_KEY"),
        "qwen" => Some("DASHSCOPE_API_KEY"),
        "ark" => Some("ARK_API_KEY"),
        "google" => Some("GOOGLE_API_KEY"),
        _ => None,
    }
}

pub(super) fn reference_present(reference: &str) -> Result<bool, AppError> {
    if let Some(env_key) = reference.strip_prefix("env:") {
        return Ok(std::env::var_os(env_key).is_some());
    }
    Ok(!reference.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::titleize;

    #[test]
    fn titleizes_registry_labels() {
        assert_eq!(titleize("workspace_models"), "Workspace Models");
    }
}
