use std::collections::HashMap;

pub fn json_to_metadata(json_str: &str) -> HashMap<String, serde_json::Value> {
    serde_json::from_str(json_str).unwrap_or_else(|_| HashMap::new())
}

pub fn metadata_to_json(metadata: &HashMap<String, serde_json::Value>) -> String {
    serde_json::to_string(metadata).unwrap_or_default()
}

pub fn update_metadata_value(
    metadata: &mut HashMap<String, serde_json::Value>,
    key: &str,
    new_value: serde_json::Value,
) {
    if let Some(existing_value) = metadata.get_mut(key) {
        *existing_value = new_value;
    } else {
        metadata.insert(key.to_string(), new_value);
    }
}

pub fn add_metadata_key(
    metadata: &mut HashMap<String, serde_json::Value>,
    key: &str,
    new_value: serde_json::Value,
) {
    metadata.insert(key.to_string(), new_value);
}

pub fn add_nested_metadata_key (
    metadata: &mut HashMap<String, serde_json::Value>,
    outer_key: &str,
    inner_key: &str,
    new_value: serde_json::Value,
) {
    if let Some(outer_value) = metadata.get_mut(outer_key) {
        if let Some(inner_map) = outer_value.as_object_mut() {
            inner_map.insert(inner_key.to_string(), new_value);
        } else {
            let mut new_inner_map = HashMap::new();
            new_inner_map.insert(inner_key.to_string(), new_value);
            *outer_value = serde_json::json!(new_inner_map);
        }
    } else {
        let mut new_inner_map = HashMap::new();
        new_inner_map.insert(inner_key.to_string(), new_value);
        metadata.insert(outer_key.to_string(), serde_json::json!(new_inner_map));
    }
}

pub fn delete_metadata_key(
    metadata: &mut HashMap<String, serde_json::Value>,
    key: &str,
) {
    metadata.remove(key);
}

pub fn delete_nested_metadata_key(
    metadata: &mut HashMap<String, serde_json::Value>,
    outer_key: &str,
    inner_key: &str
) {
    if let Some(outer_value) = metadata.get_mut(outer_key) {
        if let Some(inner_map) = outer_value.as_object_mut() {
            inner_map.remove(inner_key);
        }
    }
}
pub fn find_metadata_value<'a>(
    metadata: &'a HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<&'a serde_json::Value> {
    metadata.get(key)
}

pub fn find_nested_metadata_value<'a>(
    metadata: &'a HashMap<String, serde_json::Value>,
    outer_key: &str,
    inner_key: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(outer_value) = metadata.get(outer_key) {
        if let Some(inner_map) = outer_value.as_object() {
            return inner_map.get(inner_key);
        }
    }
    None
}
