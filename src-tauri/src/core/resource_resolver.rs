use std::path::PathBuf;

use crate::core::paths;

pub fn resolve_image(path: &str) -> PathBuf {
    let path = std::path::Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let custom = paths::user_images_dir().join(path);
    if custom.is_file() {
        custom
    } else {
        paths::built_in_images_dir().join(path)
    }
}
