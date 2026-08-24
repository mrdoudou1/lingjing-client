use crate::domain::Asset;
use std::path::{Path, PathBuf};

pub struct MediaStore {
    root: PathBuf,
}

impl Default for MediaStore {
    fn default() -> Self {
        let root = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("Lingjing")
            .join("assets");
        Self { root }
    }
}

impl MediaStore {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn save_bytes(
        &self,
        asset_id: &str,
        extension: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, String> {
        let original_dir = self.root.join("originals").join(asset_id);
        let temp_dir = self
            .root
            .parent()
            .unwrap_or(&self.root)
            .join("cache")
            .join("temp");
        std::fs::create_dir_all(&original_dir)
            .map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        std::fs::create_dir_all(&temp_dir).map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        let final_path = original_dir.join(format!("original.{extension}"));
        let temp_path = temp_dir.join(format!("{asset_id}-{}.part", uuid::Uuid::new_v4()));
        std::fs::write(&temp_path, bytes).map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        std::fs::rename(&temp_path, &final_path)
            .map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        Ok(final_path)
    }

    pub fn is_under_root(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    pub fn remove_asset_files(&self, asset: &Asset) -> Result<(), String> {
        for raw_path in [
            Some(asset.local_path.as_str()),
            asset.thumbnail_path.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if raw_path.starts_with("mock://") {
                continue;
            }
            let path = Path::new(raw_path);
            if !self.is_under_root(path) {
                return Err("STORAGE_DELETE_BLOCKED: asset path is outside the asset store".into());
            }
            if path.exists() {
                std::fs::remove_file(path).map_err(|error| format!("STORAGE_FAILED: {error}"))?;
            }
        }
        let original_dir = self.root.join("originals").join(&asset.id);
        if original_dir.exists() {
            std::fs::remove_dir(&original_dir)
                .map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        }
        Ok(())
    }

    pub fn usage_bytes(&self) -> u64 {
        fn walk(path: &Path) -> u64 {
            let Ok(entries) = std::fs::read_dir(path) else {
                return 0;
            };
            entries
                .flatten()
                .map(|entry| {
                    let child = entry.path();
                    if child.is_dir() {
                        walk(&child)
                    } else {
                        entry.metadata().map(|meta| meta.len()).unwrap_or(0)
                    }
                })
                .sum()
        }
        walk(&self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::MediaStore;
    use crate::domain::Asset;
    use std::fs;

    #[test]
    fn save_bytes_uses_temp_file_and_final_asset_path() {
        let root =
            std::env::temp_dir().join(format!("lingjing-media-test-{}", uuid::Uuid::new_v4()));
        let store = MediaStore::at(root.clone());
        let path = store
            .save_bytes("asset-1", "png", b"png-data")
            .expect("asset should save");
        assert!(path.ends_with("original.png"));
        assert_eq!(fs::read(&path).unwrap(), b"png-data");
        assert!(store.is_under_root(&path));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remove_asset_files_deletes_owned_media_and_reports_usage() {
        let root =
            std::env::temp_dir().join(format!("lingjing-media-delete-{}", uuid::Uuid::new_v4()));
        let store = MediaStore::at(root.clone());
        let path = store.save_bytes("asset-1", "png", b"png-data").unwrap();
        assert!(store.usage_bytes() >= 8);
        let asset = Asset {
            id: "asset-1".into(),
            job_id: None,
            kind: "image".into(),
            mime_type: "image/png".into(),
            local_path: path.to_string_lossy().into_owned(),
            thumbnail_path: None,
            size_bytes: 8,
            favorite: false,
            created_at: chrono::Utc::now(),
        };
        store.remove_asset_files(&asset).unwrap();
        assert!(!path.exists());
        assert_eq!(store.usage_bytes(), 0);
        fs::remove_dir_all(root).unwrap();
    }
}
