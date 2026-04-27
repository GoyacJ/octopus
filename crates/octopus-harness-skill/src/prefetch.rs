#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum SkillPrefetchStrategy {
    None,
    Lazy,
    #[default]
    Eager,
    Hybrid {
        eager_limit: usize,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SkillPrefetchPlan {
    pub load_on_session_start: bool,
    pub load_limit: Option<usize>,
}

impl SkillPrefetchStrategy {
    #[must_use]
    pub fn plan_for_skill_count(self, skill_count: usize) -> SkillPrefetchPlan {
        match self {
            Self::None => SkillPrefetchPlan {
                load_on_session_start: false,
                load_limit: Some(0),
            },
            Self::Lazy => SkillPrefetchPlan {
                load_on_session_start: false,
                load_limit: None,
            },
            Self::Eager => SkillPrefetchPlan {
                load_on_session_start: true,
                load_limit: None,
            },
            Self::Hybrid { eager_limit } => SkillPrefetchPlan {
                load_on_session_start: skill_count <= eager_limit,
                load_limit: Some(eager_limit),
            },
        }
    }
}
