use anyhow::{anyhow, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub content: Option<String>,
}

#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchResult>>;
}

pub struct TinyFishSearchProvider {
    pub api_key: String,
}

impl TinyFishSearchProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait::async_trait]
impl SearchProvider for TinyFishSearchProvider {
    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchResult>> {
        let client = reqwest::Client::new();

        let res = client
            .get("https://api.search.tinyfish.ai")
            .query(&[("query", query)])
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| anyhow!("Websearch error: {}", e))?;

        let json_resp: serde_json::Value = res
            .json()
            .await
            .map_err(|e| anyhow!("JSON parse error: {}", e))?;

        let items = if let Some(arr) = json_resp.as_array() {
            arr.clone()
        } else if let Some(arr) = json_resp.get("results").and_then(|v| v.as_array()) {
            arr.clone()
        } else if let Some(err) = json_resp
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return Err(anyhow!("TinyFish API error: {}", err));
        } else {
            return Err(anyhow!(
                "Unexpected response format from TinyFish Search API: {:?}",
                json_resp
            ));
        };

        let mut search_results = Vec::new();
        for item in items.into_iter().take(num_results) {
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let snippet = item
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            search_results.push(SearchResult {
                title,
                url,
                snippet,
                content: None,
            });
        }

        Ok(search_results)
    }
}

pub async fn fetch_url_content(url: &str, api_key: &str) -> String {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let request_body = serde_json::json!({
        "urls": [url],
        "format": "markdown"
    });

    match client
        .post("https://api.fetch.tinyfish.ai")
        .header("X-API-Key", api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json_resp) = resp.json::<serde_json::Value>().await {
                if let Some(results) = json_resp.get("results").and_then(|v| v.as_array()) {
                    if let Some(first_result) = results.first() {
                        if let Some(text) = first_result.get("text").and_then(|v| v.as_str()) {
                            return text.chars().take(4000).collect::<String>();
                        }
                    }
                } else if let Some(err) = json_resp
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    return format!("Failed to extract content from TinyFish Fetch API: {}", err);
                }
            }
            "Failed to extract content from TinyFish Fetch API".to_string()
        }
        Err(e) => format!("Failed to fetch: {}", e),
    }
}
