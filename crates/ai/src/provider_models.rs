//! Fetching a provider's available model list from its OpenAI-compatible
//! `GET /v1/models` endpoint, so users can pick from real models instead of
//! hand-typing slugs when adding a custom endpoint.
//!
//! Nearly every OpenAI-compatible provider (OpenAI, Groq, Mistral, DeepSeek,
//! Together, Fireworks, OpenRouter, ...) implements `/models` returning
//! `{ "object": "list", "data": [ { "id": "...", ... }, ... ] }`. This module
//! calls that endpoint with the user's key and returns the model ids.
//!
//! Native-only: relies on `http_client`, which isn't built for wasm.

use std::time::Duration;

use anyhow::{bail, Context as _};
use serde::Deserialize;

const MODEL_LIST_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// Fetches the sorted, de-duplicated list of model ids advertised by the
/// endpoint at `base_url` (the same base URL used for chat completions, e.g.
/// `https://api.groq.com/openai/v1`). `api_key` is sent as a bearer token.
///
/// Returns an error the caller can surface in the UI when the endpoint is
/// unreachable, rejects the key, or doesn't implement `/models`.
pub async fn fetch_models(base_url: &str, api_key: &str) -> anyhow::Result<Vec<String>> {
    let url = models_url_for_base(base_url)?;

    let client = http_client::Client::new();
    let mut request = client
        .get(&url)
        .header("Accept", "application/json")
        .timeout(MODEL_LIST_TIMEOUT);
    let api_key = api_key.trim();
    if !api_key.is_empty() {
        request = request.bearer_auth(api_key);
    }

    let response = request
        .send()
        .await
        .context("couldn't reach the provider's /models endpoint")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let hint = if status.as_u16() == 404 {
            " (this provider may not support listing models — add them manually)"
        } else {
            ""
        };
        bail!("provider returned {status}{hint}: {body}");
    }

    let parsed: ModelsResponse = response
        .json()
        .await
        .context("couldn't parse the provider's model list")?;

    let mut ids: Vec<String> = parsed
        .data
        .into_iter()
        .map(|m| m.id)
        .filter(|id| !id.trim().is_empty())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        bail!("the provider returned an empty model list");
    }
    Ok(ids)
}

fn models_url_for_base(base_url: &str) -> anyhow::Result<String> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        bail!("enter the endpoint URL first");
    }

    if base.ends_with("/models") {
        return Ok(base.to_string());
    }

    for suffix in ["/chat/completions", "/responses"] {
        if let Some(base) = base.strip_suffix(suffix) {
            return Ok(format!("{base}/models"));
        }
    }

    Ok(format!("{base}/models"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_url_appends_models_to_base_endpoint() {
        assert_eq!(
            models_url_for_base("https://api.example.com/v1").unwrap(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn models_url_reuses_existing_models_endpoint() {
        assert_eq!(
            models_url_for_base("https://api.example.com/v1/models").unwrap(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn models_url_handles_chat_completions_endpoint() {
        assert_eq!(
            models_url_for_base("https://api.example.com/v1/chat/completions").unwrap(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn models_url_handles_responses_endpoint() {
        assert_eq!(
            models_url_for_base("https://api.example.com/v1/responses").unwrap(),
            "https://api.example.com/v1/models"
        );
    }
}
