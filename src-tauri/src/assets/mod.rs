use crate::domain::Asset;
use std::collections::HashMap;

#[derive(Default)]
pub struct AssetStore {
    assets: HashMap<String, Asset>,
}
impl AssetStore {
    pub fn from_assets(assets: Vec<Asset>) -> Self {
        Self {
            assets: assets
                .into_iter()
                .map(|asset| (asset.id.clone(), asset))
                .collect(),
        }
    }

    pub fn list(&self) -> Vec<Asset> {
        self.assets.values().cloned().collect()
    }
    pub fn upsert(&mut self, asset: Asset) {
        self.assets.insert(asset.id.clone(), asset);
    }
    pub fn toggle_favorite(&mut self, id: &str) -> Option<Asset> {
        let asset = self.assets.get_mut(id)?;
        asset.favorite = !asset.favorite;
        Some(asset.clone())
    }
    pub fn delete(&mut self, id: &str) {
        self.assets.remove(id);
    }
    pub fn usage(&self) -> u64 {
        self.assets.values().map(|asset| asset.size_bytes).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::AssetStore;
    use crate::domain::Asset;
    use chrono::Utc;

    #[test]
    fn favorite_toggle_updates_asset_metadata() {
        let asset = Asset {
            id: "asset-1".into(),
            job_id: None,
            kind: "image".into(),
            mime_type: "image/png".into(),
            local_path: "mock://asset-1".into(),
            thumbnail_path: None,
            size_bytes: 10,
            favorite: false,
            created_at: Utc::now(),
        };
        let mut store = AssetStore::from_assets(vec![asset]);
        assert_eq!(
            store.toggle_favorite("asset-1").map(|item| item.favorite),
            Some(true)
        );
        assert_eq!(
            store.toggle_favorite("asset-1").map(|item| item.favorite),
            Some(false)
        );
        assert!(store.toggle_favorite("missing").is_none());
    }
}
