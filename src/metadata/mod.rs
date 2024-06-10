use serde::{Serialize, Deserialize};
// Metadata handling, serialization (JSON)

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub name: String,
    pub value: String
}

#[allow(unused_variables)]
pub fn serialize_data(data: &Data) -> String {
    // Serialize data into JSON
    todo!()
}

#[allow(unused_variables)]
pub fn deserialize_data(json_str: &str) -> Data {
    // Deserialize data from JSON
    todo!()
}
