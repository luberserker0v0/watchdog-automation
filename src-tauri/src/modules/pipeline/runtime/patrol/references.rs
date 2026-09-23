use base64::Engine as _;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use std::path::{Path, PathBuf};
#[derive(Debug, Clone)]
pub struct ReferenceStore {
    pub(super) root: PathBuf,
}

impl ReferenceStore {
    pub fn for_run(pipeline_path: &Path, run_id: &str) -> Result<Self, String> {
        Self::for_run_in(
            crate::core::paths::visions_dir().join("patrol"),
            pipeline_path,
            run_id,
        )
    }

    pub(super) fn for_run_in(
        base: PathBuf,
        pipeline_path: &Path,
        run_id: &str,
    ) -> Result<Self, String> {
        if run_id.is_empty()
            || !run_id
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err("Invalid patrol run ID".to_string());
        }
        let canonical = pipeline_path.canonicalize().map_err(|error| {
            format!(
                "Cannot resolve patrol pipeline {}: {error}",
                pipeline_path.display()
            )
        })?;
        // Stable FNV-1a namespacing avoids exposing full local paths on disk.
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in canonical.to_string_lossy().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Ok(Self {
            root: base.join(format!("{hash:016x}")).join(run_id),
        })
    }

    pub(super) fn node_path(&self, node_id: &str) -> Result<PathBuf, String> {
        if node_id.is_empty()
            || !node_id.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == '-'
            })
        {
            return Err(format!("Invalid patrol node ID '{node_id}'"));
        }
        Ok(self.root.join(format!("{node_id}.jpg")))
    }

    pub fn load(&self, node_id: &str) -> Result<Option<Vec<u8>>, String> {
        let path = self.node_path(node_id)?;
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!(
                "Failed to read patrol reference {}: {error}",
                path.display()
            )),
        }
    }

    pub fn save(&self, node_id: &str, frame: &DynamicImage) -> Result<(String, Vec<u8>), String> {
        let path = self.node_path(node_id)?;
        std::fs::create_dir_all(&self.root)
            .map_err(|error| format!("Failed to create patrol reference directory: {error}"))?;
        let bytes = encode_frame(frame)?;
        std::fs::write(&path, &bytes).map_err(|error| {
            format!(
                "Failed to save patrol reference {}: {error}",
                path.display()
            )
        })?;
        Ok((path.display().to_string(), bytes))
    }
}

pub(super) fn encode_frame(frame: &DynamicImage) -> Result<Vec<u8>, String> {
    let resized = frame.resize(1280, 1280, image::imageops::FilterType::Triangle);
    let mut output = Cursor::new(Vec::new());
    resized
        .write_to(&mut output, ImageFormat::Jpeg)
        .map_err(|error| format!("Failed to encode patrol frame: {error}"))?;
    Ok(output.into_inner())
}

pub(super) fn image_data_url(bytes: &[u8]) -> String {
    format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}
