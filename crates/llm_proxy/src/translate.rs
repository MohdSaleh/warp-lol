//! Request/response translation between the OpenAI Chat Completions shape and
//! each subscription provider's native protocol.
//!
//! The proxy accepts an OpenAI `chat/completions` JSON body and, per provider:
//!
//!   * [`ProviderProtocol::OpenAiCompatible`] — forward almost verbatim, only
//!     swapping in the OAuth bearer token and any provider headers (GitHub
//!     Copilot, Qwen Code).
//!   * [`ProviderProtocol::Anthropic`] — convert to the Anthropic Messages API
//!     and back (Claude Code).
//!   * [`ProviderProtocol::OpenAiResponses`] — convert to the OpenAI Responses
//!     API used by the ChatGPT/Codex backend and back.
//!   * [`ProviderProtocol::GeminiCodeAssist`] — convert to Google Code Assist
//!     `:generateContent` and back (Gemini CLI).
//!
//! This module implements non-streaming translation (the proxy requests a
//! non-streamed upstream response and returns a single Chat Completions object).
//! Streaming can be layered on later; Warp tolerates non-streamed endpoints.

use serde_json::{json, Value};

use crate::{ProviderCredentials, ProviderRoute};

/// Which upstream wire protocol a provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    /// Already OpenAI-shaped; forward with token + headers.
    OpenAiCompatible,
    /// Anthropic Messages API (`/v1/messages`).
    Anthropic,
    /// OpenAI Responses API (`/responses`), as used by the ChatGPT/Codex backend.
    OpenAiResponses,
    /// Google Code Assist `:generateContent`.
    GeminiCodeAssist,
}

/// Failure modes when forwarding to an upstream provider.
pub enum ForwardError {
    /// The upstream returned a non-success status; surface it to the caller.
    Upstream {
        status: u16,
        body: String,
    },
    Other(anyhow::Error),
}

impl From<anyhow::Error> for ForwardError {
    fn from(err: anyhow::Error) -> Self {
        ForwardError::Other(err)
    }
}

/// Forwards one OpenAI chat completion request to `route`'s upstream, returning
/// an OpenAI-shaped completion object.
pub async fn forward_chat_completion(
    http: &reqwest::Client,
    route: &ProviderRoute,
    creds: &ProviderCredentials,
    request: Value,
) -> Result<Value, ForwardError> {
    match route.protocol {
        ProviderProtocol::OpenAiCompatible => {
            forward_openai_compatible(http, route, creds, request).await
        }
        ProviderProtocol::Anthropic => forward_anthropic(http, route, creds, request).await,
        ProviderProtocol::OpenAiResponses => {
            forward_openai_responses(http, route, creds, request).await
        }
        ProviderProtocol::GeminiCodeAssist => forward_gemini(http, route, creds, request).await,
    }
}

fn model_of(request: &Value) -> String {
    request
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Reads OpenAI `messages` as `(role, text)` pairs, flattening string or
/// content-array message bodies into plain text (sufficient for chat/coding).
fn messages_as_text(request: &Value) -> Vec<(String, String)> {
    request
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .map(|message| {
                    let role = message
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or("user")
                        .to_string();
                    (role, content_to_text(message.get("content")))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn content_to_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// Builds a minimal OpenAI Chat Completions response object around `text`.
fn openai_completion(model: &str, text: &str) -> Value {
    json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": text },
            "finish_reason": "stop"
        }],
    })
}

async fn read_response(response: reqwest::Response) -> Result<Value, ForwardError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ForwardError::Other(e.into()))?;
    if !status.is_success() {
        return Err(ForwardError::Upstream {
            status: status.as_u16(),
            body,
        });
    }
    serde_json::from_str(&body).map_err(|e| ForwardError::Other(e.into()))
}

/// Returns the exact model ID passed by the user without normalization.
pub fn normalize_model_id(_provider_id: &str, model: &str) -> String {
    model.to_string()
}

// ── OpenAI-compatible passthrough (Copilot, Qwen Code, NVIDIA NIM) ─────────

async fn forward_openai_compatible(
    http: &reqwest::Client,
    route: &ProviderRoute,
    creds: &ProviderCredentials,
    mut request: Value,
) -> Result<Value, ForwardError> {
    // Force non-streaming so we can return a single JSON object.
    let mut is_minimax = false;
    if let Value::Object(map) = &mut request {
        map.insert("stream".into(), Value::Bool(false));

        if let Some(model_val) = map.get("model").and_then(Value::as_str) {
            let normalized = normalize_model_id(&route.provider_id, model_val);
            is_minimax = normalized.contains("minimax");
            map.insert("model".into(), Value::String(normalized));
        }

        // Inject reasoning/thinking parameters into API request for NVIDIA NIM reasoning models
        if route.provider_id == "nvidia_nim" || route.provider_id.contains("nvidia") {
            let extra_body = map
                .entry("extra_body".to_string())
                .or_insert_with(|| json!({}));
            if let Value::Object(extra_map) = extra_body {
                let kwargs = extra_map
                    .entry("chat_template_kwargs".to_string())
                    .or_insert_with(|| json!({}));
                if let Value::Object(kwargs_map) = kwargs {
                    kwargs_map.insert("thinking".to_string(), Value::Bool(true));
                }
            }
        }

        // For non-native tool calling models (e.g. MiniMax), inject tools into system prompt & strip `tools` param
        if is_minimax {
            if let Some(tools) = map.remove("tools") {
                let tools_json = serde_json::to_string(&tools).unwrap_or_default();
                if let Some(Value::Array(messages)) = map.get_mut("messages") {
                    messages.insert(
                        0,
                        json!({
                            "role": "system",
                            "content": format!(
                                "Available tools: {tools_json}. To invoke a tool, respond ONLY with JSON: {{\x22name\x22: \x22tool_name\x22, \x22arguments\x22: {{...}}}}"
                            )
                        }),
                    );
                }
            }
        }
    }

    let url = format!(
        "{}/chat/completions",
        route.upstream_base_url.trim_end_matches('/')
    );
    let mut builder = http
        .post(&url)
        .bearer_auth(&creds.access_token)
        .header("Content-Type", "application/json");

    // GitHub Copilot requires an editor identity header and integration id.
    if route.provider_id == "github_copilot" {
        builder = builder
            .header("Copilot-Integration-Id", "vscode-chat")
            .header("Editor-Version", "Warp/1.0")
            .header("Editor-Plugin-Version", "warp-llm-proxy/0.1");
    }

    let response = builder
        .json(&request)
        .send()
        .await
        .map_err(|e| ForwardError::Other(e.into()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ForwardError::Other(e.into()))?;

    if !status.is_success() {
        if status == reqwest::StatusCode::NOT_FOUND
            && body.contains("Function")
            && body.contains("Not found for account")
        {
            return Ok(json!({
                "error": {
                    "message": "NVIDIA NIM Account Entitlement Error: Public API Endpoints permission is not enabled for your account for this model. Contact help@build.nvidia.com to enable it.",
                    "type": "invalid_request_error",
                    "code": "model_entitlement_required"
                }
            }));
        }
        return Err(ForwardError::Upstream {
            status: status.as_u16(),
            body,
        });
    }

    let mut res: Value = serde_json::from_str(&body).map_err(|e| ForwardError::Other(e.into()))?;

    // If non-native model printed tool JSON in content text, parse it into structured tool_calls using rig_adapter
    if is_minimax {
        if let Some(choices) = res.get_mut("choices").and_then(Value::as_array_mut) {
            for choice in choices {
                if let Some(msg) = choice.get_mut("message") {
                    if let Some(content) = msg.get("content").and_then(Value::as_str) {
                        if let Some(tool_calls) =
                            crate::rig_adapter::extract_tool_calls_from_text(content)
                        {
                            msg.as_object_mut()
                                .unwrap()
                                .insert("tool_calls".into(), Value::Array(tool_calls));
                            choice
                                .as_object_mut()
                                .unwrap()
                                .insert("finish_reason".into(), json!("tool_calls"));
                        }
                    }
                }
            }
        }
    }

    Ok(res)
}

// ── Anthropic Messages (Claude Code) ────────────────────────────────────────

async fn forward_anthropic(
    http: &reqwest::Client,
    route: &ProviderRoute,
    creds: &ProviderCredentials,
    request: Value,
) -> Result<Value, ForwardError> {
    let model = model_of(&request);
    let mut system = String::new();
    let mut messages = Vec::new();
    for (role, text) in messages_as_text(&request) {
        if role == "system" {
            if !system.is_empty() {
                system.push_str("\n\n");
            }
            system.push_str(&text);
        } else {
            let role = if role == "assistant" {
                "assistant"
            } else {
                "user"
            };
            messages.push(json!({
                "role": role,
                "content": [{ "type": "text", "text": text }],
            }));
        }
    }

    let max_tokens = request
        .get("max_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(4096);
    let mut body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = Value::String(system);
    }

    let url = format!(
        "{}/v1/messages",
        route.upstream_base_url.trim_end_matches('/')
    );
    let response = http
        .post(&url)
        // Claude subscription OAuth tokens authenticate via Bearer + the
        // OAuth beta header (rather than the x-api-key used by API keys).
        .bearer_auth(&creds.access_token)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ForwardError::Other(e.into()))?;

    let value = read_response(response).await?;
    // Anthropic returns { content: [ { type: "text", text } ], ... }.
    let text = value
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(openai_completion(&model, &text))
}

// ── OpenAI Responses (ChatGPT / Codex) ──────────────────────────────────────

async fn forward_openai_responses(
    http: &reqwest::Client,
    route: &ProviderRoute,
    creds: &ProviderCredentials,
    request: Value,
) -> Result<Value, ForwardError> {
    let model = model_of(&request);
    // The Responses API takes a flat `input` array of role/content items.
    let input: Vec<Value> = messages_as_text(&request)
        .into_iter()
        .map(|(role, text)| {
            json!({
                "role": role,
                "content": [{ "type": "input_text", "text": text }],
            })
        })
        .collect();

    let body = json!({
        "model": model,
        "input": input,
        "stream": false,
    });

    let url = format!(
        "{}/responses",
        route.upstream_base_url.trim_end_matches('/')
    );
    let mut builder = http
        .post(&url)
        .bearer_auth(&creds.access_token)
        .header("Content-Type", "application/json")
        .header("OpenAI-Beta", "responses=v1");
    if let Some(account_id) = &creds.account_id {
        builder = builder.header("chatgpt-account-id", account_id);
    }

    let response = builder
        .json(&body)
        .send()
        .await
        .map_err(|e| ForwardError::Other(e.into()))?;
    let value = read_response(response).await?;

    // Prefer the convenience `output_text`; else walk `output[].content[].text`.
    let text = value
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| extract_responses_text(&value));
    Ok(openai_completion(&model, &text))
}

fn extract_responses_text(value: &Value) -> String {
    value
        .get("output")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("content").and_then(Value::as_array))
                .flatten()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

// ── Google Code Assist (Gemini CLI) ─────────────────────────────────────────

async fn forward_gemini(
    http: &reqwest::Client,
    route: &ProviderRoute,
    creds: &ProviderCredentials,
    request: Value,
) -> Result<Value, ForwardError> {
    let model = model_of(&request);
    let mut system_instruction = String::new();
    let mut contents = Vec::new();
    for (role, text) in messages_as_text(&request) {
        if role == "system" {
            if !system_instruction.is_empty() {
                system_instruction.push_str("\n\n");
            }
            system_instruction.push_str(&text);
            continue;
        }
        let gemini_role = if role == "assistant" { "model" } else { "user" };
        contents.push(json!({
            "role": gemini_role,
            "parts": [{ "text": text }],
        }));
    }

    let mut inner = json!({ "contents": contents });
    if !system_instruction.is_empty() {
        inner["systemInstruction"] = json!({ "parts": [{ "text": system_instruction }] });
    }

    // Code Assist wraps the GenerateContent request with project + model.
    let mut body = json!({
        "model": model,
        "request": inner,
    });
    if let Some(project) = &creds.account_id {
        body["project"] = Value::String(project.clone());
    }

    let url = format!(
        "{}/v1internal:generateContent",
        route.upstream_base_url.trim_end_matches('/')
    );
    let response = http
        .post(&url)
        .bearer_auth(&creds.access_token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ForwardError::Other(e.into()))?;
    let value = read_response(response).await?;

    // Response: { response: { candidates: [ { content: { parts: [ {text} ] }}]}}
    let text = value
        .get("response")
        .unwrap_or(&value)
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(openai_completion(&model, &text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> Value {
        json!({
            "model": "test-model",
            "messages": [
                { "role": "system", "content": "be terse" },
                { "role": "user", "content": "hi" },
                { "role": "assistant", "content": [{ "type": "text", "text": "yo" }] }
            ]
        })
    }

    #[test]
    fn flattens_messages() {
        let msgs = messages_as_text(&sample_request());
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], ("system".into(), "be terse".into()));
        assert_eq!(msgs[2], ("assistant".into(), "yo".into()));
    }

    #[test]
    fn openai_completion_shape() {
        let c = openai_completion("m", "hello");
        assert_eq!(c["object"], "chat.completion");
        assert_eq!(c["choices"][0]["message"]["content"], "hello");
        assert_eq!(c["choices"][0]["message"]["role"], "assistant");
    }

    #[test]
    fn extracts_responses_text_from_output() {
        let v = json!({
            "output": [
                { "content": [ { "type": "output_text", "text": "part1 " },
                               { "type": "output_text", "text": "part2" } ] }
            ]
        });
        assert_eq!(extract_responses_text(&v), "part1 part2");
    }
}
