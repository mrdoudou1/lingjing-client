use crate::domain::Asset;
use std::collections::HashMap;

#[derive(Default)]
pub struct AssetStore {
    assets: HashMap<String, Asset>,
}
impl AssetStore {
    pub fn list(&self) -> Vec<Asset> {
        self.assets.values().cloned().collect()
    }
    pub fn delete(&mut self, id: &str) {
        self.assets.remove(id);
    }
    pub fn usage(&self) -> u64 {
        self.assets.values().map(|asset| asset.size_bytes).sum()
    }
}
