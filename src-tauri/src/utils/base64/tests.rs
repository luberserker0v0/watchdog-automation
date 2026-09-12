use super::*;

#[test]
fn test_buf_to_base64_empty() {
    let result = buf_to_base64(&[]);
    assert_eq!(result, "data:image/png;base64,");
}

#[test]
fn test_buf_to_base64_single_byte() {
    let result = buf_to_base64(&[0xFF]);
    assert_eq!(result, "data:image/png;base64,/w==");
}

#[test]
fn test_buf_to_base64_png_header() {
    let buf = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let result = buf_to_base64(&buf);
    assert!(result.starts_with("data:image/png;base64,"));
    assert!(result.len() > 30);
}

#[test]
fn test_buf_to_base64_output_format() {
    let result = buf_to_base64(&[1, 2, 3]);
    assert!(result.starts_with("data:image/png;base64,"));
}
