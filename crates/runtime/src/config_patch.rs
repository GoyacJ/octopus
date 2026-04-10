use std::collections::BTreeMap;

use crate::json::JsonValue;

pub(crate) fn apply_config_patch(
    target: &mut BTreeMap<String, JsonValue>,
    patch: &BTreeMap<String, JsonValue>,
) {
    for (key, value) in patch {
        match value {
            JsonValue::Null => {
                target.remove(key);
            }
            JsonValue::Object(incoming) => {
                if let Some(JsonValue::Object(existing)) = target.get_mut(key) {
                    apply_config_patch(existing, incoming);
                } else {
                    let mut next = BTreeMap::new();
                    apply_config_patch(&mut next, incoming);
                    target.insert(key.clone(), JsonValue::Object(next));
                }
            }
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}
