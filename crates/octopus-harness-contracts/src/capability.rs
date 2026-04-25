//! Tool capability marker traits.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.4

pub trait SubagentRunnerCap: Send + Sync + 'static {}
pub trait TodoStoreCap: Send + Sync + 'static {}
pub trait RunCancellerCap: Send + Sync + 'static {}
pub trait ClarifyChannelCap: Send + Sync + 'static {}
pub trait UserMessengerCap: Send + Sync + 'static {}
pub trait BlobReaderCap: Send + Sync + 'static {}
pub trait HookEmitterCap: Send + Sync + 'static {}
pub trait SkillRegistryCap: Send + Sync + 'static {}
pub trait EmbeddedToolDispatcherCap: Send + Sync + 'static {}
pub trait CodeRuntimeCap: Send + Sync + 'static {}
