use std::path::PathBuf;

use crate::utils::paths;

pub fn resolve_image(path: &str) -> PathBuf {
    paths::resource_images_dir().join(path)
}
