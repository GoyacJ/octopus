use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn new_v4() -> Self {
                Self(Uuid::new_v4().to_string())
            }
        }
    };
}

define_id!(SessionId);
define_id!(RunId);
define_id!(ToolCallId);
define_id!(EventId);

#[cfg(test)]
mod tests {
    use super::SessionId;

    #[test]
    fn session_id_new_v4_generates_non_empty_values() {
        let first = SessionId::new_v4();
        let second = SessionId::new_v4();

        assert!(!first.0.is_empty());
        assert!(!second.0.is_empty());
        assert_ne!(first.0, second.0);
    }
}
