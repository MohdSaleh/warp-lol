//! App-side glue for the local translation proxy (`llm_proxy`).
//!
//! Connects the [`ApiKeyManager`]'s stored CLI-subscription OAuth tokens to a
//! locally-running [`LlmProxy`], so a connected subscription (ChatGPT/Codex,
//! Gemini CLI, Claude Code, GitHub Copilot, Qwen Code) can be driven through an
//! OpenAI-compatible custom endpoint.
//!
//! Responsibilities:
//!   * own the [`SharedCredentials`] snapshot the proxy reads from its request
//!     threads, and keep it in sync with the model-layer [`ApiKeyManager`],
//!   * lazily start the proxy the first time a subscription is connected,
//!   * hand out the loopback base URL a Warp custom endpoint should target.
//!
//! The model layer (`ApiKeyManager`) is single-threaded/main-thread, while the
//! proxy serves requests on a tokio runtime; [`SharedCredentials`] is the
//! thread-safe handoff between them.

use std::sync::Arc;

use ai::api_keys::{ApiKeyManager, CliSubscriptionTokens};
use ai::cli_subscription::providers::CliProvider;
use llm_proxy::{
    LlmProxy, ProviderCredentials, ProviderProtocol, ProviderRoute, SharedCredentials,
};
use strum::IntoEnumIterator;
use warpui::{Entity, ModelContext, SingletonEntity};

/// Singleton managing the lifecycle of the local translation proxy.
pub struct SubscriptionProxyManager {
    credentials: SharedCredentials,
    proxy: Option<LlmProxy>,
}

impl SubscriptionProxyManager {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            credentials: SharedCredentials::new(),
            proxy: None,
        }
    }

    /// The proxy route table: one entry per CLI-subscription provider.
    fn provider_routes() -> Vec<ProviderRoute> {
        CliProvider::iter()
            .map(|provider| ProviderRoute {
                provider_id: provider.id().to_string(),
                upstream_base_url: provider.upstream_base_url().to_string(),
                protocol: protocol_for(provider),
                model_slugs: provider
                    .available_models()
                    .iter()
                    .map(|(slug, _alias)| slug.to_string())
                    .collect(),
            })
            .collect()
    }

    /// Starts the proxy if it isn't already running. Returns the handle.
    pub fn ensure_started(&mut self) -> Option<&LlmProxy> {
        if self.proxy.is_none() {
            match LlmProxy::start(Self::provider_routes(), Arc::new(self.credentials.clone())) {
                Ok(proxy) => self.proxy = Some(proxy),
                Err(err) => log::error!("Failed to start LLM subscription proxy: {err:#}"),
            }
        }
        self.proxy.as_ref()
    }

    /// The loopback base URL a custom endpoint should point at for `provider`,
    /// starting the proxy on first use.
    pub fn base_url_for(&mut self, provider: CliProvider) -> Option<String> {
        self.ensure_started()
            .map(|proxy| proxy.base_url_for(provider.id()))
    }

    /// Pushes one provider's current credentials into the snapshot the proxy
    /// reads. Passing `None` (or expired-empty tokens) clears them.
    pub fn push_credentials(&self, provider: CliProvider, tokens: Option<&CliSubscriptionTokens>) {
        let creds = tokens
            .and_then(CliSubscriptionTokens::access_token_for_request)
            .map(|token| ProviderCredentials {
                access_token: token.to_string(),
                account_id: tokens.and_then(|t| t.account_id.clone()),
            });
        self.credentials.set(provider.id(), creds);
    }

    /// Re-syncs every provider's credentials from the [`ApiKeyManager`]. Call
    /// on startup (to load restored tokens) and whenever tokens change.
    pub fn sync_all(&self, api_keys: &ApiKeyManager) {
        for provider in CliProvider::iter() {
            self.push_credentials(provider, api_keys.cli_subscription(provider.id()));
        }
    }
}

/// Maps a provider to the upstream wire protocol the proxy translates into.
fn protocol_for(provider: CliProvider) -> ProviderProtocol {
    match provider {
        CliProvider::ChatgptCodex => ProviderProtocol::OpenAiResponses,
        CliProvider::GeminiCli => ProviderProtocol::GeminiCodeAssist,
        CliProvider::ClaudeCode => ProviderProtocol::Anthropic,
        CliProvider::GithubCopilot => ProviderProtocol::OpenAiCompatible,
        CliProvider::QwenCode => ProviderProtocol::OpenAiCompatible,
    }
}

impl Entity for SubscriptionProxyManager {
    type Event = ();
}

impl SingletonEntity for SubscriptionProxyManager {}
