//! A local, OpenAI-compatible **translation proxy** for subscription/OAuth
//! providers.
//!
//! # Why this exists
//!
//! Warp's BYO "custom endpoints" can talk to anything that speaks the OpenAI
//! Chat Completions API with a static bearer key. Subscription providers
//! (ChatGPT/Codex, Gemini CLI, Claude Code, GitHub Copilot, Qwen Code) don't:
//! they authenticate with rotating OAuth tokens and, in several cases, speak a
//! different wire protocol (OpenAI Responses, Google `generateContent`,
//! Anthropic Messages). This crate bridges the gap.
//!
//! The proxy binds `127.0.0.1:<port>` and exposes:
//!
//! ```text
//! POST /{provider_id}/v1/chat/completions
//! ```
//!
//! A user adds one Warp custom endpoint per provider pointing at
//! `http://127.0.0.1:<port>/{provider_id}/v1`. On each request the proxy:
//!
//!   1. looks up the connected subscription's fresh OAuth access token via the
//!      injected [`TokenSource`],
//!   2. rewrites the OpenAI request into the provider's native protocol,
//!   3. forwards it upstream with the token, and
//!   4. rewrites the response back into OpenAI shape.
//!
//! # Reachability
//!
//! Because it lives on loopback, only a *local* consumer can reach it. When
//! Warp's hosted inference server makes the model request, expose the proxy via
//! an HTTPS tunnel and point the custom endpoint at the tunnel URL instead.
//! Self-hosted Warp servers on the same machine can use the loopback URL
//! directly.

pub mod rig_adapter;
mod translate;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use tower_http::cors::CorsLayer;
pub use translate::ProviderProtocol;

/// A fresh set of credentials for one connected subscription provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderCredentials {
    pub access_token: String,
    /// Provider-specific account/project id, when required (ChatGPT account id,
    /// Google Cloud project, ...).
    pub account_id: Option<String>,
}

/// Supplies fresh OAuth credentials for a provider slug at request time.
///
/// The app implements this over `ai::api_keys::ApiKeyManager`, taking care to
/// return a non-expired access token (kicking off a refresh when needed). The
/// proxy never persists tokens itself.
pub trait TokenSource: Send + Sync + 'static {
    /// Returns credentials for `provider_id`, or `None` if not connected.
    fn credentials(&self, provider_id: &str) -> Option<ProviderCredentials>;
}

/// A thread-safe credential snapshot the app can update from the UI/main thread
/// while the proxy reads it from its request threads.
///
/// The app's model layer (`ApiKeyManager`) is not `Send`/thread-shareable, so
/// rather than reaching into it from the proxy, the app pushes the current
/// tokens here whenever they change (connect, disconnect, background refresh).
#[derive(Clone, Default)]
pub struct SharedCredentials {
    inner: Arc<std::sync::RwLock<HashMap<String, ProviderCredentials>>>,
}

impl SharedCredentials {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or clears (`None`) credentials for a provider.
    pub fn set(&self, provider_id: &str, creds: Option<ProviderCredentials>) {
        let mut map = self.inner.write().expect("credential lock poisoned");
        match creds {
            Some(creds) => {
                map.insert(provider_id.to_string(), creds);
            }
            None => {
                map.remove(provider_id);
            }
        }
    }
}

impl TokenSource for SharedCredentials {
    fn credentials(&self, provider_id: &str) -> Option<ProviderCredentials> {
        self.inner
            .read()
            .expect("credential lock poisoned")
            .get(provider_id)
            .cloned()
    }
}

/// Static configuration for one routable provider on the proxy.
#[derive(Debug, Clone)]
pub struct ProviderRoute {
    /// URL path segment and token-source key (e.g. "chatgpt_codex").
    pub provider_id: String,
    /// Upstream base URL for this provider.
    pub upstream_base_url: String,
    /// Which native protocol to translate into.
    pub protocol: ProviderProtocol,
    /// Model slugs this provider supports, returned by `/v1/models`.
    pub model_slugs: Vec<String>,
}

/// Shared proxy state handed to every request handler.
struct ProxyState {
    routes: HashMap<String, ProviderRoute>,
    tokens: Arc<dyn TokenSource>,
    http: reqwest::Client,
}

/// A running proxy handle. Dropping it shuts the server down.
pub struct LlmProxy {
    addr: SocketAddr,
    _runtime: tokio::runtime::Runtime,
}

impl LlmProxy {
    /// Builds the axum router without binding a socket. Exposed so it can be
    /// merged into an existing server (like Warp's `http_server`) instead of
    /// running standalone.
    pub fn router(routes: Vec<ProviderRoute>, tokens: Arc<dyn TokenSource>) -> Router {
        let routes = routes
            .into_iter()
            .map(|route| (route.provider_id.clone(), route))
            .collect();
        let state = Arc::new(ProxyState {
            routes,
            tokens,
            http: reqwest::Client::new(),
        });
        Router::new()
            .route(
                "/{provider_id}/v1/chat/completions",
                post(handle_chat_completions),
            )
            .route(
                "/{provider_id}/v1/models",
                axum::routing::get(handle_models),
            )
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    /// Starts the proxy on its own tokio runtime, bound to an OS-assigned
    /// loopback port. Returns the handle (with [`LlmProxy::base_url_for`]).
    pub fn start(routes: Vec<ProviderRoute>, tokens: Arc<dyn TokenSource>) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;

        let router = Self::router(routes, tokens);
        let (addr_tx, addr_rx) = std::sync::mpsc::channel();

        runtime.spawn(async move {
            let listener = match tokio::net::TcpListener::bind(("127.0.0.1", 0)).await {
                Ok(l) => l,
                Err(err) => {
                    log::error!("llm_proxy failed to bind: {err}");
                    let _ = addr_tx.send(Err(err));
                    return;
                }
            };
            let addr = listener.local_addr();
            let _ = addr_tx.send(addr);
            if let Err(err) = axum::serve(listener, router).await {
                log::error!("llm_proxy exited: {err}");
            }
        });

        let addr = addr_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("llm_proxy server thread stopped before binding"))??;
        log::info!("llm_proxy listening on http://{addr}");
        Ok(Self {
            addr,
            _runtime: runtime,
        })
    }

    /// The base URL a Warp custom endpoint should target for `provider_id`.
    pub fn base_url_for(&self, provider_id: &str) -> String {
        format!("http://{}/{provider_id}/v1", self.addr)
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

/// A JSON error body in OpenAI's `{ "error": { ... } }` shape.
fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let body = serde_json::json!({
        "error": { "message": message.into(), "type": "proxy_error" }
    });
    (status, Json(body)).into_response()
}

async fn handle_chat_completions(
    State(state): State<Arc<ProxyState>>,
    Path(provider_id): Path<String>,
    Json(request): Json<Value>,
) -> Response {
    let Some(route) = state.routes.get(&provider_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            format!("unknown provider {provider_id}"),
        );
    };
    let Some(creds) = state.tokens.credentials(&provider_id) else {
        return error_response(
            StatusCode::UNAUTHORIZED,
            format!("{provider_id} is not connected; connect it in Warp settings"),
        );
    };

    match translate::forward_chat_completion(&state.http, route, &creds, request).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(translate::ForwardError::Upstream { status, body }) => {
            let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
            (status, body).into_response()
        }
        Err(translate::ForwardError::Other(err)) => {
            error_response(StatusCode::BAD_GATEWAY, format!("{err:#}"))
        }
    }
}

/// `/v1/models` handler. Returns the models configured for this provider so
/// the client can discover which model slugs are usable through the proxy.
async fn handle_models(
    State(state): State<Arc<ProxyState>>,
    Path(provider_id): Path<String>,
) -> Response {
    let models: Vec<Value> = state
        .routes
        .get(&provider_id)
        .map(|route| {
            route
                .model_slugs
                .iter()
                .map(|slug| {
                    serde_json::json!({
                        "id": slug,
                        "object": "model",
                        "owned_by": provider_id,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let body = serde_json::json!({
        "object": "list",
        "data": models,
    });
    (StatusCode::OK, Json(body)).into_response()
}
