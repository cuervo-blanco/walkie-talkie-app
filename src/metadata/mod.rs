use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use serde_json::json;
// Metadata handling, serialization (JSON)


#[allow(unused_variables)]
pub fn serialize_hashmap(map: HashMap<String, String>) -> String {
    // Serialize data into JSON
    serde_json::to_string(&map).unwrap()
}

#[allow(unused_variables)]
pub fn deserialize_data(json_str: &str) -> Data {
    // Deserialize data from JSON
    todo!()
}
