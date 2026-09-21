use super::*;
use tempfile::tempdir;

#[test]
fn test_save_to_existing_dir() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.png");
    let buf = vec![0x89, 0x50, 0x4E, 0x47];

    save_buf_to_file(&buf, path.to_str().unwrap()).unwrap();

    let saved = std::fs::read(&path).unwrap();
    assert_eq!(saved, buf);
}

#[test]
fn test_save_creates_parent_dirs() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sub").join("dir").join("test.png");

    save_buf_to_file(&[1, 2, 3], path.to_str().unwrap()).unwrap();

    assert!(path.exists());
}

#[test]
fn test_save_empty_buf() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.png");

    save_buf_to_file(&[], path.to_str().unwrap()).unwrap();

    let saved = std::fs::read(&path).unwrap();
    assert!(saved.is_empty());
}

#[test]
fn test_save_overwrites_existing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.png");

    save_buf_to_file(&[1, 2, 3], path.to_str().unwrap()).unwrap();
    save_buf_to_file(&[4, 5, 6], path.to_str().unwrap()).unwrap();

    let saved = std::fs::read(&path).unwrap();
    assert_eq!(saved, vec![4, 5, 6]);
}
