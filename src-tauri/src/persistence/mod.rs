use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default)]
pub struct SettingsStore {
    values: HashMap<String, serde_json::Value>,
}
impl SettingsStore {
    pub fn get(&self) -> HashMap<String, serde_json::Value> {
        self.values.clone()
    }
    pub fn update(&mut self, values: HashMap<String, serde_json::Value>) {
        self.values.extend(values);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub values: HashMap<String, serde_json::Value>,
}
