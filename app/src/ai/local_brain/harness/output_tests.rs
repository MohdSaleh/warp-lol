use super::*;

#[test]
fn short_content_is_unchanged() {
    let content = "small observation".to_string();
    assert_eq!(bound("read_files", content.clone()), content);
}

#[test]
fn oversized_content_spills_to_a_readable_file() {
    let content = "x".repeat(MAX_INLINE_CHARS + 5_000);

    let bounded = bound("grep", content.clone());

    let marker_start = bounded.find("… [truncated:").expect("truncation marker");
    let path_start = bounded.find("full output saved to ").expect("spill path") + "full output saved to ".len();
    let path = &bounded[path_start..bounded.len() - 1];

    assert!(bounded.starts_with(&content[..MAX_INLINE_CHARS]));
    assert!(marker_start >= MAX_INLINE_CHARS);
    assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    let _ = std::fs::remove_file(path);
}

#[test]
fn tool_names_are_sanitized_for_file_names() {
    let content = "y".repeat(MAX_INLINE_CHARS + 1);
    let bounded = bound("mcp/../../evil", content);

    let path_start = bounded.find("full output saved to ").expect("spill path") + "full output saved to ".len();
    let path = &bounded[path_start..bounded.len() - 1];

    let name = std::path::Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(!name.contains('/'), "path separator leaked into: {name}");
    assert!(!name.contains(".."), "traversal leaked into: {name}");
    assert!(name.ends_with("evil.txt"), "unexpected file name: {name}");
    let _ = std::fs::remove_file(path);
}
