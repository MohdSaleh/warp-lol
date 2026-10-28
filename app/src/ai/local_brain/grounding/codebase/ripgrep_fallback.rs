use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(not(test))]
use warp_ripgrep::search::search;

#[cfg(test)]
async fn search(
    queries: &[String],
    paths: &[std::path::PathBuf],
    _ignore_case: bool,
    _multiline: bool,
) -> anyhow::Result<Vec<warp_ripgrep::search::Match>> {
    let mut matches = Vec::new();
    let query = &queries[0];

    // Simple mock: if query is UNIQUE_SYMBOL_XYZ, return a fake match for file2.rs
    if query == "UNIQUE_SYMBOL_XYZ" {
        matches.push(warp_ripgrep::search::Match {
            file_path: paths[0].join("file2.rs"),
            line_number: 1,
            line_text: "fn UNIQUE_SYMBOL_XYZ() {}".to_string(),
            submatches: vec![],
        });
    }

    Ok(matches)
}

use futures::future::BoxFuture;

use super::lexical_fallback::{
    CodeSearchFallback, LexicalSearchError, LexicalSearchRequest, LexicalSearchResult,
};

pub struct RipgrepCodeSearchFallback {
    root_path: PathBuf,
}

impl RipgrepCodeSearchFallback {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }
}

impl CodeSearchFallback for RipgrepCodeSearchFallback {
    fn search(
        &self,
        request: LexicalSearchRequest,
    ) -> BoxFuture<'static, Result<LexicalSearchResult, LexicalSearchError>> {
        let root_path = self.root_path.clone();
        let query = request.query;

        Box::pin(async move {
            let matches = search(&[query], std::slice::from_ref(&root_path), true, false)
                .await
                .map_err(|_| LexicalSearchError)?;

            let mut match_counts: HashMap<PathBuf, usize> = HashMap::new();
            for m in matches {
                *match_counts.entry(m.file_path).or_insert(0) += 1;
            }

            let mut files_with_counts: Vec<(PathBuf, usize)> = match_counts.into_iter().collect();
            files_with_counts.sort_by(|a, b| b.1.cmp(&a.1));

            let files = files_with_counts
                .into_iter()
                .take(20)
                .filter_map(|(path, _)| {
                    path.strip_prefix(&root_path)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string())
                })
                .collect();

            Ok(LexicalSearchResult { files })
        })
    }
}

#[cfg(test)]
#[path = "ripgrep_fallback_tests.rs"]
mod tests;
