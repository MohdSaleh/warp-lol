use std::path::PathBuf;

use tempfile::tempdir;

use super::{CodeSearchFallback, LexicalSearchRequest, RipgrepCodeSearchFallback};

#[tokio::test]
async fn test_lexical_search() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();

    // Create a few files
    let file1 = root.join("file1.rs");
    std::fs::write(&file1, "fn some_function() {}\n").unwrap();

    let file2 = root.join("file2.rs");
    std::fs::write(&file2, "fn UNIQUE_SYMBOL_XYZ() {}\n").unwrap();

    let fallback = RipgrepCodeSearchFallback::new(root.to_path_buf());
    let req = LexicalSearchRequest {
        query: "UNIQUE_SYMBOL_XYZ".to_string(),
    };

    let result = fallback.search(req).await.unwrap();
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].contains("file2.rs"));
}
