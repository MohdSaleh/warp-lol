use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;

use super::super::filesystem_registry;
use super::super::super::executor::{HarnessToolCall, ToolContext, ToolOutcomeStatus};

fn call(name: &str, arguments: serde_json::Value) -> HarnessToolCall {
    HarnessToolCall {
        call_id: "call-1".to_string(),
        name: name.to_string(),
        arguments,
    }
}

fn context(dir: &Path) -> ToolContext {
    ToolContext {
        working_directory: Some(dir.to_path_buf()),
        turn: 1,
    }
}

#[tokio::test]
async fn read_files_reads_relative_paths() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "hello from a").unwrap();
    fs::create_dir_all(temp.path().join("nested")).unwrap();
    fs::write(temp.path().join("nested/b.txt"), "hello from b").unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("read_files", json!({ "paths": ["a.txt", "nested/b.txt"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("hello from a"));
    assert!(outcome.content.contains("hello from b"));
    assert!(outcome.content.contains("a.txt"));
}

#[tokio::test]
async fn read_files_accepts_the_files_alias() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "aliased").unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("read_files", json!({ "files": ["a.txt"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("aliased"));
}

#[tokio::test]
async fn read_files_reports_missing_files_alongside_successes() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), "present").unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("read_files", json!({ "paths": ["a.txt", "missing.txt"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("present"));
    assert!(outcome.content.contains("missing.txt"));
    assert!(outcome.content.contains("Could not read"));
}

#[tokio::test]
async fn read_files_fails_when_nothing_could_be_read() {
    let temp = TempDir::new().unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("read_files", json!({ "paths": ["missing.txt"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("missing.txt"));
}

#[tokio::test]
async fn read_files_rejects_binary_files() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("bin.dat"), [0u8, 1, 2, 3]).unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("read_files", json!({ "paths": ["bin.dat"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("binary"));
}

#[tokio::test]
async fn read_files_requires_paths() {
    let temp = TempDir::new().unwrap();

    let outcome = filesystem_registry()
        .execute(&call("read_files", json!({})), &context(temp.path()))
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("missing required field: paths"));
}

#[tokio::test]
async fn read_files_pages_with_numbered_lines() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("notes.txt"),
        "one\ntwo\nthree\nfour\nfive\n",
    )
    .unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call(
                "read_files",
                json!({ "paths": ["notes.txt"], "offset": 2, "limit": 2 }),
            ),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("notes.txt (lines 2-3 of 5)"));
    assert!(outcome.content.contains("2: two"));
    assert!(outcome.content.contains("3: three"));
    assert!(!outcome.content.contains("1: one"));
    assert!(outcome.content.contains("pass offset=4 to continue"));
}

#[tokio::test]
async fn read_files_rejects_offsets_past_the_end() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("short.txt"), "only line\n").unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("read_files", json!({ "paths": ["short.txt"], "offset": 9 })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("out of range"));
}

#[tokio::test]
async fn glob_matches_nested_patterns_and_skips_vendor_dirs() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("target")).unwrap();
    fs::write(temp.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(temp.path().join("src/lib.rs"), "").unwrap();
    fs::write(temp.path().join("target/generated.rs"), "").unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("glob", json!({ "patterns": ["**/*.rs"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("main.rs"));
    assert!(outcome.content.contains("lib.rs"));
    assert!(!outcome.content.contains("generated.rs"));
}

#[tokio::test]
async fn glob_rejects_invalid_patterns() {
    let temp = TempDir::new().unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("glob", json!({ "patterns": ["[unterminated"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("invalid glob pattern"));
}

#[tokio::test]
async fn grep_returns_matching_lines_with_text() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("notes.txt"),
        "first line\nsecond needle line\nthird line\n",
    )
    .unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("grep", json!({ "queries": ["needle"] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("\"line_number\":2"));
    assert!(outcome.content.contains("second needle line"));
}

#[tokio::test]
async fn grep_falls_back_to_literal_matching_for_invalid_regex() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("code.txt"), "call foo(1) here\n").unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("grep", json!({ "queries": ["foo("] })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("call foo(1) here"));
}

#[tokio::test]
async fn grep_reports_missing_search_paths() {
    let temp = TempDir::new().unwrap();

    let outcome = filesystem_registry()
        .execute(
            &call("grep", json!({ "queries": ["x"], "path": "does-not-exist" })),
            &context(temp.path()),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("does not exist"));
}

#[tokio::test]
async fn unknown_tools_are_reported_by_the_shared_registry() {
    let temp = TempDir::new().unwrap();

    let outcome = filesystem_registry()
        .execute(&call("run_command", json!({})), &context(temp.path()))
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert!(outcome.content.contains("unsupported tool: run_command"));
}
