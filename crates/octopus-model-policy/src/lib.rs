use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalDefaultSelection {
    pub purpose: &'static str,
    pub provider_id: &'static str,
    pub model_id: &'static str,
    pub surface: &'static str,
    pub protocol_family: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalModelAlias {
    pub alias: &'static str,
    pub model_id: &'static str,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CanonicalModelPolicy;

const CONVERSATION_DEFAULT: CanonicalDefaultSelection = CanonicalDefaultSelection {
    purpose: "conversation",
    provider_id: "anthropic",
    model_id: "claude-sonnet-4-5",
    surface: "conversation",
    protocol_family: "anthropic_messages",
};

const RESPONSES_DEFAULT: CanonicalDefaultSelection = CanonicalDefaultSelection {
    purpose: "responses",
    provider_id: "openai",
    model_id: "gpt-5.4",
    surface: "responses",
    protocol_family: "openai_responses",
};

const FAST_DEFAULT: CanonicalDefaultSelection = CanonicalDefaultSelection {
    purpose: "fast",
    provider_id: "openai",
    model_id: "gpt-5.4-mini",
    surface: "responses",
    protocol_family: "openai_responses",
};

const DEFAULT_SELECTIONS: &[CanonicalDefaultSelection] =
    &[CONVERSATION_DEFAULT, RESPONSES_DEFAULT, FAST_DEFAULT];

const MODEL_ALIASES: &[CanonicalModelAlias] = &[
    CanonicalModelAlias {
        alias: "opus",
        model_id: "claude-opus-4-6",
    },
    CanonicalModelAlias {
        alias: "sonnet",
        model_id: "claude-sonnet-4-5",
    },
    CanonicalModelAlias {
        alias: "haiku",
        model_id: "claude-haiku-4-5-20251213",
    },
    CanonicalModelAlias {
        alias: "grok",
        model_id: "grok-3",
    },
    CanonicalModelAlias {
        alias: "grok-3",
        model_id: "grok-3",
    },
    CanonicalModelAlias {
        alias: "grok-mini",
        model_id: "grok-3-mini",
    },
    CanonicalModelAlias {
        alias: "grok-3-mini",
        model_id: "grok-3-mini",
    },
    CanonicalModelAlias {
        alias: "grok-2",
        model_id: "grok-2",
    },
];

impl CanonicalModelPolicy {
    #[must_use]
    pub const fn default_conversation_model(self) -> &'static str {
        CONVERSATION_DEFAULT.model_id
    }

    #[must_use]
    pub const fn default_responses_model(self) -> &'static str {
        RESPONSES_DEFAULT.model_id
    }

    #[must_use]
    pub const fn default_fast_model(self) -> &'static str {
        FAST_DEFAULT.model_id
    }

    #[must_use]
    pub const fn default_conversation_selection(self) -> CanonicalDefaultSelection {
        CONVERSATION_DEFAULT
    }

    #[must_use]
    pub const fn default_responses_selection(self) -> CanonicalDefaultSelection {
        RESPONSES_DEFAULT
    }

    #[must_use]
    pub const fn default_fast_selection(self) -> CanonicalDefaultSelection {
        FAST_DEFAULT
    }

    #[must_use]
    pub const fn default_selections(self) -> &'static [CanonicalDefaultSelection] {
        DEFAULT_SELECTIONS
    }

    #[must_use]
    pub fn default_selection(self, purpose: &str) -> Option<CanonicalDefaultSelection> {
        self.default_selections()
            .iter()
            .copied()
            .find(|selection| selection.purpose == purpose)
    }

    #[must_use]
    pub fn resolve_alias(self, model: &str) -> Option<&'static str> {
        let normalized = model.trim().to_ascii_lowercase();
        MODEL_ALIASES
            .iter()
            .find(|entry| entry.alias == normalized)
            .map(|entry| entry.model_id)
    }

    #[must_use]
    pub fn canonical_model(self, model: &str) -> Cow<'_, str> {
        match self.resolve_alias(model) {
            Some(canonical) => Cow::Borrowed(canonical),
            None => Cow::Borrowed(model.trim()),
        }
    }
}
