use super::*;

const AUTH_RATE_LIMIT_WINDOW_SECONDS: u64 = 10 * 60;
const AUTH_RATE_LIMIT_MAX_FAILURES: usize = 5;
const AUTH_RATE_LIMIT_LOCK_SECONDS: u64 = 15 * 60;

fn auth_source_fingerprint(headers: &HeaderMap) -> String {
    [
        "x-forwarded-for",
        "x-real-ip",
        "cf-connecting-ip",
        "user-agent",
    ]
    .iter()
    .find_map(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
    .unwrap_or_else(|| "unknown".into())
}

pub(crate) fn auth_rate_limit_key(
    workspace_id: &str,
    username: &str,
    headers: &HeaderMap,
) -> String {
    format!(
        "{workspace_id}:{}:{}",
        username.trim().to_lowercase(),
        auth_source_fingerprint(headers)
    )
}

pub(crate) fn check_auth_rate_limit(
    state: &ServerState,
    key: &str,
) -> Result<Option<u64>, ApiError> {
    let now = timestamp_now();
    let mut rate_limits = state
        .auth_rate_limits
        .lock()
        .map_err(|_| ApiError::from(AppError::runtime("auth rate-limit mutex poisoned")))?;
    let Some(entry) = rate_limits.get_mut(key) else {
        return Ok(None);
    };
    entry
        .failed_attempts
        .retain(|attempt| now.saturating_sub(*attempt) <= AUTH_RATE_LIMIT_WINDOW_SECONDS);
    if let Some(locked_until) = entry.locked_until {
        if locked_until > now {
            return Ok(Some(locked_until));
        }
        entry.locked_until = None;
        entry.failed_attempts.clear();
    }
    Ok(None)
}

pub(crate) fn record_auth_failure(state: &ServerState, key: &str) -> Result<Option<u64>, ApiError> {
    let now = timestamp_now();
    let mut rate_limits = state
        .auth_rate_limits
        .lock()
        .map_err(|_| ApiError::from(AppError::runtime("auth rate-limit mutex poisoned")))?;
    let entry = rate_limits.entry(key.to_string()).or_default();
    entry
        .failed_attempts
        .retain(|attempt| now.saturating_sub(*attempt) <= AUTH_RATE_LIMIT_WINDOW_SECONDS);
    entry.failed_attempts.push(now);
    if entry.failed_attempts.len() >= AUTH_RATE_LIMIT_MAX_FAILURES {
        let locked_until = now + AUTH_RATE_LIMIT_LOCK_SECONDS;
        entry.locked_until = Some(locked_until);
        entry.failed_attempts.clear();
        return Ok(Some(locked_until));
    }
    Ok(None)
}

pub(crate) fn clear_auth_failures(state: &ServerState, key: &str) -> Result<bool, ApiError> {
    let mut rate_limits = state
        .auth_rate_limits
        .lock()
        .map_err(|_| ApiError::from(AppError::runtime("auth rate-limit mutex poisoned")))?;
    Ok(rate_limits.remove(key).is_some())
}
