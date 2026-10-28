use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::RegexSet;
use serde_json::{json, Value};

use super::super::executor::{HarnessToolCall, HarnessToolExecutor, ToolContext, ToolOutcome};

/// Files larger than this are reported as errors instead of being read: a
/// single huge file would otherwise consume the model's whole context.
const MAX_FILE_BYTES: u64 = 512 * 1024;
const MAX_GREP_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_GLOB_MATCHES: usize = 200;
const MAX_GREP_FILES: usize = 100;
const MAX_GREP_LINES_PER_FILE: usize = 50;
const MAX_LINE_CHARS: usize = 500;
const MAX_READ_LINES: usize = 2_000;
const MAX_READ_LINE_CHARS: usize = 2_000;
const BINARY_SNIFF_BYTES: usize = 8 * 1024;
const MAX_WALK_DEPTH: usize = 64;

/// Directories that never hold user source and would dominate the walk.
const SKIPPED_DIRS: &[&str] = &[".git", "target", "node_modules", ".venv", "dist", "build"];

fn string_list(arguments: &Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(items) = arguments.get(key).and_then(Value::as_array) {
            let values: Vec<String> = items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            if !values.is_empty() {
                return values;
            }
        }
    }
    Vec::new()
}

fn resolve(context: &ToolContext, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return path;
    }
    match &context.working_directory {
        Some(dir) => dir.join(path),
        None => path,
    }
}

fn search_root(context: &ToolContext, arguments: &Value) -> PathBuf {
    match arguments.get("path").and_then(Value::as_str) {
        Some(raw) => resolve(context, raw),
        None => context
            .working_directory
            .clone()
            .unwrap_or_else(|| PathBuf::from(".")),
    }
}

fn is_probably_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_SNIFF_BYTES).any(|byte| *byte == 0)
}

fn truncate_line(line: &str) -> String {
    if line.chars().count() <= MAX_LINE_CHARS {
        return line.to_string();
    }
    line.chars().take(MAX_LINE_CHARS).collect::<String>() + "…"
}

fn truncate_read_line(line: &str) -> String {
    if line.chars().count() <= MAX_READ_LINE_CHARS {
        return line.to_string();
    }
    line.chars().take(MAX_READ_LINE_CHARS).collect::<String>() + "… (line truncated)"
}

fn render_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

/// One file as the model sees it: numbered lines, the requested window, and a
/// footer that says how to page the rest. A window that starts past the end is
/// an error for that file, not a silently empty result.
fn render_read(path: &str, content: &str, offset: usize, limit: usize) -> Result<String, String> {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    if total == 0 {
        if offset > 1 {
            return Err(format!("offset {offset} is out of range for this file (0 lines)"));
        }
        return Ok(format!("{path} (empty file)"));
    }
    if offset > total {
        return Err(format!("offset {offset} is out of range for this file ({total} lines)"));
    }

    let start = offset;
    let end = (start - 1 + limit).min(total);
    let mut rendered = format!("{path} (lines {start}-{end} of {total}):\n");
    for (index, line) in lines[start - 1..end].iter().enumerate() {
        rendered.push_str(&format!("{}: {}\n", start + index, truncate_read_line(line)));
    }
    if end < total {
        rendered.push_str(&format!(
            "… [showing lines {start}-{end} of {total}; pass offset={} to continue]",
            end + 1
        ));
    }
    Ok(rendered.trim_end().to_string())
}

fn skip_entry(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    match entry.file_name().to_str() {
        Some(name) => !SKIPPED_DIRS.contains(&name),
        None => false,
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .max_depth(MAX_WALK_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(skip_entry)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

fn render_errors(header: &str, errors: &[String]) -> String {
    let mut rendered = format!("{header}:");
    for error in errors {
        rendered.push_str("\n- ");
        rendered.push_str(error);
    }
    rendered
}

pub struct ReadFilesExecutor;

#[async_trait]
impl HarnessToolExecutor for ReadFilesExecutor {
    fn tool_name(&self) -> &str {
        "read_files"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, call: &HarnessToolCall, context: &ToolContext) -> ToolOutcome {
        let paths = string_list(&call.arguments, &["paths", "files"]);
        if paths.is_empty() {
            return ToolOutcome::error("missing required field: paths");
        }
        // One page window applies to every file in the call.
        let offset = call
            .arguments
            .get("offset")
            .and_then(Value::as_u64)
            .map_or(1, |value| value.max(1) as usize);
        let limit = call
            .arguments
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(MAX_READ_LINES, |value| {
                (value as usize).clamp(1, MAX_READ_LINES)
            });

        let resolved: Vec<PathBuf> = paths
            .iter()
            .map(|raw| resolve(context, raw))
            .collect();

        let join = tokio::task::spawn_blocking(move || {
            let mut files: Vec<(String, String)> = Vec::new();
            let mut errors: Vec<String> = Vec::new();
            for path in resolved {
                let display = path.display().to_string();
                match fs::metadata(&path) {
                    Ok(metadata) if metadata.is_file() => {
                        if metadata.len() > MAX_FILE_BYTES {
                            errors.push(format!(
                                "{display}: file is {} bytes, over the {MAX_FILE_BYTES} byte read limit",
                                metadata.len()
                            ));
                            continue;
                        }
                    }
                    Ok(_) => {
                        errors.push(format!("{display}: path is not a regular file"));
                        continue;
                    }
                    Err(error) => {
                        errors.push(format!("{display}: {error}"));
                        continue;
                    }
                }

                match fs::read(&path) {
                    Ok(bytes) if is_probably_binary(&bytes) => {
                        errors.push(format!("{display}: file appears to be binary"))
                    }
                    Ok(bytes) => files.push((display, String::from_utf8_lossy(&bytes).into_owned())),
                    Err(error) => errors.push(format!("{display}: {error}")),
                }
            }
            (files, errors)
        })
        .await;

        let Ok((files, errors)) = join else {
            return ToolOutcome::error("read_files worker failed");
        };

        if files.is_empty() {
            return ToolOutcome::error(render_errors("read_files could not read any path", &errors));
        }

        let mut rendered = Vec::new();
        let mut page_errors = Vec::new();
        for (path, content) in files {
            match render_read(&path, &content, offset, limit) {
                Ok(text) => rendered.push(text),
                Err(message) => page_errors.push(format!("{path}: {message}")),
            }
        }
        if rendered.is_empty() {
            return ToolOutcome::error(render_errors(
                "read_files could not read any path",
                &page_errors,
            ));
        }

        let mut output = rendered.join("\n\n");
        let mut unreadable = errors;
        unreadable.append(&mut page_errors);
        if !unreadable.is_empty() {
            output.push_str("\n\n");
            output.push_str(&render_errors("Could not read", &unreadable));
        }
        ToolOutcome::success(output)
    }
}

pub struct GlobExecutor;

#[async_trait]
impl HarnessToolExecutor for GlobExecutor {
    fn tool_name(&self) -> &str {
        "glob"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, call: &HarnessToolCall, context: &ToolContext) -> ToolOutcome {
        let patterns = string_list(&call.arguments, &["patterns"]);
        if patterns.is_empty() {
            return ToolOutcome::error("missing required field: patterns");
        }

        let mut builder = GlobSetBuilder::new();
        for pattern in &patterns {
            match Glob::new(pattern) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(error) => {
                    return ToolOutcome::error(format!("invalid glob pattern {pattern:?}: {error}"))
                }
            }
        }
        let glob_set: GlobSet = match builder.build() {
            Ok(set) => set,
            Err(error) => return ToolOutcome::error(format!("failed to build glob matcher: {error}")),
        };

        let root = search_root(context, &call.arguments);
        let join = tokio::task::spawn_blocking(move || {
            let mut warnings = Vec::new();
            if !root.exists() {
                return (Vec::new(), vec![format!("search path does not exist: {}", root.display())]);
            }

            let mut matched = Vec::new();
            for path in walk_files(&root) {
                let relative = path.strip_prefix(&root).unwrap_or(&path);
                if glob_set.is_match(relative) {
                    matched.push(path.display().to_string());
                    if matched.len() >= MAX_GLOB_MATCHES {
                        warnings.push(format!(
                            "stopped after {MAX_GLOB_MATCHES} matches; narrow the pattern to see more"
                        ));
                        break;
                    }
                }
            }
            (matched, warnings)
        })
        .await;

        let Ok((matched_files, warnings)) = join else {
            return ToolOutcome::error("glob worker failed");
        };

        ToolOutcome::success(render_json(&json!({
            "matched_files": matched_files,
            "warnings": warnings,
        })))
    }
}

pub struct GrepExecutor;

#[async_trait]
impl HarnessToolExecutor for GrepExecutor {
    fn tool_name(&self) -> &str {
        "grep"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, call: &HarnessToolCall, context: &ToolContext) -> ToolOutcome {
        let queries = string_list(&call.arguments, &["queries"]);
        if queries.is_empty() {
            return ToolOutcome::error("missing required field: queries");
        }

        let matcher = match RegexSet::new(&queries) {
            Ok(set) => set,
            Err(_) => {
                // Queries read as plain text are common; retry as literals
                // before surfacing a regex error to the model.
                let escaped: Vec<String> = queries
                    .iter()
                    .map(|query| regex::escape(query))
                    .collect();
                match RegexSet::new(&escaped) {
                    Ok(set) => set,
                    Err(error) => {
                        return ToolOutcome::error(format!("invalid grep pattern: {error}"))
                    }
                }
            }
        };

        let root = search_root(context, &call.arguments);
        let join = tokio::task::spawn_blocking(move || {
            if !root.exists() {
                return Err(format!("search path does not exist: {}", root.display()));
            }

            let candidates = if root.is_file() {
                vec![root.clone()]
            } else {
                walk_files(&root)
            };

            let mut hits = Vec::new();
            for path in candidates {
                if hits.len() >= MAX_GREP_FILES {
                    break;
                }
                let Ok(metadata) = fs::metadata(&path) else {
                    continue;
                };
                if metadata.len() > MAX_GREP_FILE_BYTES {
                    continue;
                }
                let Ok(bytes) = fs::read(&path) else {
                    continue;
                };
                if is_probably_binary(&bytes) {
                    continue;
                }

                let content = String::from_utf8_lossy(&bytes);
                let mut lines = Vec::new();
                for (index, line) in content.lines().enumerate() {
                    if matcher.is_match(line) {
                        lines.push(json!({
                            "line_number": index + 1,
                            "line": truncate_line(line),
                        }));
                        if lines.len() >= MAX_GREP_LINES_PER_FILE {
                            break;
                        }
                    }
                }
                if !lines.is_empty() {
                    hits.push(json!({
                        "file_path": path.display().to_string(),
                        "lines": lines,
                    }));
                }
            }
            Ok(hits)
        })
        .await;

        match join {
            Ok(Ok(hits)) => ToolOutcome::success(render_json(&json!({ "hits": hits }))),
            Ok(Err(error)) => ToolOutcome::error(error),
            Err(_) => ToolOutcome::error("grep worker failed"),
        }
    }
}

#[cfg(test)]
#[path = "fs_tests.rs"]
mod tests;
