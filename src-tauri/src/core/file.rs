use std::path::Path;

pub fn save_buf_to_file(buf: &[u8], path: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, buf)
}

#[cfg(test)]
mod tests;
