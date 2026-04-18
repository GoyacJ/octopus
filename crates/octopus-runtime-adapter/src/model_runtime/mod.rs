mod auth;
mod canonical_model_policy;
mod driver;
mod driver_registry;
mod drivers;
mod request_policy;
mod simple_completion;

pub(crate) use auth::{
    parse_model_credential_reference, validate_runtime_credential_reference, CredentialReference,
    CREDENTIAL_SOURCE_CONFIGURED_MODEL_OVERRIDE, CREDENTIAL_SOURCE_PROBE_OVERRIDE,
    CREDENTIAL_SOURCE_PROVIDER_INHERITED, CREDENTIAL_SOURCE_UNCONFIGURED,
};
pub use auth::{resolve_model_auth_source, ResolvedModelAuth, ResolvedModelAuthMode};
pub use canonical_model_policy::{
    CanonicalDefaultSelection, CanonicalModelAlias, CanonicalModelPolicy,
};
pub use driver::{
    LiveRuntimeModelDriver, MockRuntimeModelDriver, ModelExecutionDeliverable,
    ModelExecutionResult, ProtocolDriver, ProtocolDriverCapability, RuntimeConversationExecution,
    RuntimeConversationRequest, RuntimeModelDriver,
};
pub use driver_registry::ModelDriverRegistry;
pub(crate) use request_policy::resolve_request_base_url;
pub use request_policy::resolve_request_policy;
