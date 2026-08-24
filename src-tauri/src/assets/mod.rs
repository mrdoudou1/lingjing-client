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
}
