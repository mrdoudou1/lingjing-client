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
        let temp_path = temp_dir.join(format!("{asset_id}.part"));
        std::fs::write(&temp_path, bytes).map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        std::fs::rename(&temp_path, &final_path)
            .map_err(|error| format!("STORAGE_FAILED: {error}"))?;
        Ok(final_path)
    }

    pub fn is_under_root(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::MediaStore;
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
}
