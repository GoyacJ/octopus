use super::anthropic_oauth::{load_saved_oauth_token, oauth_token_is_expired};
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthSource {
    None,
    ApiKey(String),
    BearerToken(String),
    ApiKeyAndBearer {
        api_key: String,
        bearer_token: String,
    },
}

impl AuthSource {
    pub fn from_env() -> Result<Self, ApiError> {
        let api_key = read_env_non_empty("ANTHROPIC_API_KEY")?;
        let auth_token = read_env_non_empty("ANTHROPIC_AUTH_TOKEN")?;
        match (api_key, auth_token) {
            (Some(api_key), Some(bearer_token)) => Ok(Self::ApiKeyAndBearer {
                api_key,
                bearer_token,
            }),
            (Some(api_key), None) => Ok(Self::ApiKey(api_key)),
            (None, Some(bearer_token)) => Ok(Self::BearerToken(bearer_token)),
            (None, None) => Err(anthropic_missing_credentials()),
        }
    }

    #[must_use]
    pub fn api_key(&self) -> Option<&str> {
        match self {
            Self::ApiKey(api_key) | Self::ApiKeyAndBearer { api_key, .. } => Some(api_key),
            Self::None | Self::BearerToken(_) => None,
        }
    }

    #[must_use]
    pub fn bearer_token(&self) -> Option<&str> {
        match self {
            Self::BearerToken(token)
            | Self::ApiKeyAndBearer {
                bearer_token: token,
                ..
            } => Some(token),
            Self::None | Self::ApiKey(_) => None,
        }
    }

    #[must_use]
    pub fn masked_authorization_header(&self) -> &'static str {
        if self.bearer_token().is_some() {
            "Bearer [REDACTED]"
        } else {
            "<absent>"
        }
    }

    pub fn apply(&self, mut request_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(api_key) = self.api_key() {
            request_builder = request_builder.header("x-api-key", api_key);
        }
        if let Some(token) = self.bearer_token() {
            request_builder = request_builder.bearer_auth(token);
        }
        request_builder
    }

    pub fn from_env_or_saved() -> Result<Self, ApiError> {
        if let Some(api_key) = read_env_non_empty("ANTHROPIC_API_KEY")? {
            return match read_env_non_empty("ANTHROPIC_AUTH_TOKEN")? {
                Some(bearer_token) => Ok(Self::ApiKeyAndBearer {
                    api_key,
                    bearer_token,
                }),
                None => Ok(Self::ApiKey(api_key)),
            };
        }
        if let Some(bearer_token) = read_env_non_empty("ANTHROPIC_AUTH_TOKEN")? {
            return Ok(Self::BearerToken(bearer_token));
        }
        match load_saved_oauth_token() {
            Ok(Some(token_set)) if oauth_token_is_expired(&token_set) => {
                if token_set.refresh_token.is_some() {
                    Err(ApiError::Auth(
                        "saved OAuth token is expired; load runtime OAuth config to refresh it"
                            .to_string(),
                    ))
                } else {
                    Err(ApiError::ExpiredOAuthToken)
                }
            }
            Ok(Some(token_set)) => Ok(Self::BearerToken(token_set.access_token)),
            Ok(None) => Err(anthropic_missing_credentials()),
            Err(error) => Err(error),
        }
    }
}

const SK_ANT_BEARER_HINT: &str = "sk-ant-* keys go in ANTHROPIC_API_KEY (x-api-key header), not ANTHROPIC_AUTH_TOKEN (Bearer header). Move your key to ANTHROPIC_API_KEY.";

pub(super) fn enrich_bearer_auth_error(error: ApiError, auth: &AuthSource) -> ApiError {
    let ApiError::Api {
        status,
        error_type,
        message,
        request_id,
        body,
        retryable,
    } = error
    else {
        return error;
    };
    if status.as_u16() != 401 {
        return ApiError::Api {
            status,
            error_type,
            message,
            request_id,
            body,
            retryable,
        };
    }
    let Some(bearer_token) = auth.bearer_token() else {
        return ApiError::Api {
            status,
            error_type,
            message,
            request_id,
            body,
            retryable,
        };
    };
    if !bearer_token.starts_with("sk-ant-") || auth.api_key().is_some() {
        return ApiError::Api {
            status,
            error_type,
            message,
            request_id,
            body,
            retryable,
        };
    }
    let enriched_message = match message {
        Some(existing) => Some(format!("{existing} — hint: {SK_ANT_BEARER_HINT}")),
        None => Some(format!("hint: {SK_ANT_BEARER_HINT}")),
    };
    ApiError::Api {
        status,
        error_type,
        message: enriched_message,
        request_id,
        body,
        retryable,
    }
}

#[cfg(test)]
pub(super) fn read_api_key() -> Result<String, ApiError> {
    let auth = AuthSource::from_env_or_saved()?;
    auth.api_key()
        .or_else(|| auth.bearer_token())
        .map(ToOwned::to_owned)
        .ok_or_else(anthropic_missing_credentials)
}

#[cfg(test)]
pub(super) fn read_auth_token() -> Option<String> {
    read_env_non_empty("ANTHROPIC_AUTH_TOKEN")
        .ok()
        .and_then(std::convert::identity)
}
