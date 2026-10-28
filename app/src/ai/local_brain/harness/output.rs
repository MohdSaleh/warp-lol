//! Keeps every tool observation inside the model's context budget without
//! losing information: output over the inline limit is written to a retained
//! local file, and the inline copy ends with the path so the model can page
//! the full result back with `read_files`.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_INLINE_CHARS: usize = 24_000;

const SPILL_DIR: &str = "warp-harness-tool-output";

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Bound one observation. Short output is returned unchanged.
pub fn bound(tool: &str, content: String) -> String {
    let total = content.chars().count();
    if total <= MAX_INLINE_CHARS {
        return content;
    }
    let head: String = content.chars().take(MAX_INLINE_CHARS).collect();
    match spill(tool, &content) {
        Some(path) => format!(
            "{head}\n… [truncated: showing first {MAX_INLINE_CHARS} of {total} chars; full output saved to {}]",
            path.display()
        ),
        None => format!("{head}\n… [truncated: showing first {MAX_INLINE_CHARS} of {total} chars]"),
    }
}

fn spill(tool: &str, content: &str) -> Option<PathBuf> {
    let directory = std::env::temp_dir().join(SPILL_DIR);
    fs::create_dir_all(&directory).ok()?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = directory.join(format!("{stamp}-{sequence}-{}.txt", sanitize(tool)));
    fs::write(&path, content).ok()?;
    Some(path)
}

fn sanitize(tool: &str) -> String {
    let filtered: String = tool
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if filtered.is_empty() {
        "tool".to_string()
    } else {
        filtered
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
