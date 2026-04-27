use std::path::PathBuf;

use harness_contracts::{ThreatCategory, TrustLevel};

use crate::{SkillError, SkillPlatform, SkillSource};

#[derive(Debug, Clone, PartialEq)]
pub struct SkillRejection {
    pub source: SkillSource,
    pub raw_path: Option<PathBuf>,
    pub reason: SkillRejectReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkillRejectReason {
    ParseFrontmatter(String),
    PlatformMismatch {
        required: Vec<SkillPlatform>,
    },
    ThreatDetected {
        pattern_id: String,
        category: ThreatCategory,
    },
    NameTooLong(usize),
    DescriptionTooLong(usize),
    HookTransportNotPermitted {
        trust: TrustLevel,
    },
    Duplicate,
    Io(String),
}

impl SkillRejectReason {
    pub(crate) fn from_error(error: &SkillError) -> Self {
        match error {
            SkillError::ParseFrontmatter(message) => Self::ParseFrontmatter(message.clone()),
            SkillError::PlatformMismatch { required } => Self::PlatformMismatch {
                required: required.clone(),
            },
            SkillError::ThreatDetected {
                pattern_id,
                category,
            } => Self::ThreatDetected {
                pattern_id: pattern_id.clone(),
                category: *category,
            },
            SkillError::NameTooLong(size) => Self::NameTooLong(*size),
            SkillError::DescriptionTooLong(size) => Self::DescriptionTooLong(*size),
            SkillError::HookTransportNotPermitted { trust } => {
                Self::HookTransportNotPermitted { trust: *trust }
            }
            SkillError::Duplicate(name) => Self::ParseFrontmatter(format!("duplicate: {name}")),
            SkillError::MissingParam(name) => {
                Self::ParseFrontmatter(format!("missing param: {name}"))
            }
            SkillError::Io(error) => Self::Io(error.to_string()),
        }
    }
}
