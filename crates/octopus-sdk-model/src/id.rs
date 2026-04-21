//! Model-layer identifiers land here in Task 2.

use serde::{Deserialize, Serialize};

macro_rules! define_model_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(pub String);
    };
}

define_model_id!(ProviderId);
define_model_id!(SurfaceId);
define_model_id!(ModelId);

#[cfg(test)]
mod tests {
    use serde_json::{from_str, to_string};

    use super::{ModelId, ProviderId, SurfaceId};

    #[test]
    fn provider_surface_and_model_ids_roundtrip_json_strings() {
        let provider = ProviderId("anthropic".to_string());
        let surface = SurfaceId("conversation".to_string());
        let model = ModelId("claude-opus-4-6".to_string());

        assert_eq!(to_string(&provider).unwrap(), "\"anthropic\"");
        assert_eq!(to_string(&surface).unwrap(), "\"conversation\"");
        assert_eq!(to_string(&model).unwrap(), "\"claude-opus-4-6\"");

        let decoded_provider: ProviderId = from_str("\"anthropic\"").unwrap();
        let decoded_surface: SurfaceId = from_str("\"conversation\"").unwrap();
        let decoded_model: ModelId = from_str("\"claude-opus-4-6\"").unwrap();

        assert_eq!(decoded_provider.0, provider.0);
        assert_eq!(decoded_surface.0, surface.0);
        assert_eq!(decoded_model.0, model.0);
    }
}
