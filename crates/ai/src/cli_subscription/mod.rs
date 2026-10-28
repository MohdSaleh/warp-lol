//! Connecting CLI-agent subscriptions (ChatGPT/Codex, Gemini CLI, Claude
//! Code, GitHub Copilot, Qwen Code, ...) to Warp via their OAuth flows, so a
//! user with, say, a ChatGPT Plus or Gemini AI Pro plan can drive Warp's agent
//! through that subscription instead of pasting a pay-as-you-go API key.
//!
//! # How this fits together
//!
//! These providers do **not** expose a plain OpenAI-compatible endpoint that
//! Warp's inference server can call with a static key. Each speaks its own
//! authenticated protocol (OpenAI Responses API with a ChatGPT OAuth token,
//! Google Code Assist `generateContent`, Anthropic Messages with a Claude
//! subscription token, GitHub Copilot's chat endpoint, ...). To bridge them,
//! Warp bundles a **local translation proxy** (the `llm_proxy` crate) that:
//!
//!   1. exposes a localhost OpenAI-compatible `/v1/chat/completions` endpoint,
//!   2. translates each request into the target provider's native protocol,
//!   3. injects the connected subscription's OAuth access token, refreshing it
//!      when it is near expiry.
//!
//! This module owns only the **connect/refresh** side: the per-provider OAuth
//! configuration, running the browser Authorization-Code-with-PKCE (or
//! device-code) flow, and the token refresh lifecycle. The resulting tokens
//! are stored in [`crate::api_keys::ApiKeyManager`] under the
//! `CliSubscriptionOAuthTokens` secure-storage key.
//!
//! # Provider reachability caveat
//!
//! Because the proxy runs on `127.0.0.1`, it can only serve model requests when
//! the component making the model request is also on the user's machine (self-
//! hosted Warp server, or a tunnel exposing the proxy). For the default hosted
//! Warp inference server, localhost is not reachable; those users should point
//! the corresponding custom endpoint at a public HTTPS tunnel to the proxy.
//! The GitHub Copilot path is the exception: its chat completions endpoint is
//! itself broadly OpenAI-shaped, so a Copilot token can be used more directly.
//!
//! # ToS
//!
//! Using a subscription's OAuth token through third-party software may be
//! restricted by the provider's Terms of Service. The UI surfaces this and the
//! feature is opt-in per provider. Warp does not transmit these tokens to its
//! servers.

pub mod oauth;
pub mod providers;

use std::time::{Duration, SystemTime};

use warpui_core::r#async::Timer;
use warpui_core::ModelContext;

use self::oauth::TokenResponse;
use self::providers::CliProvider;
use crate::api_keys::{ApiKeyManager, CliSubscriptionTokens};

/// Refresh a CLI-subscription access token this long before its hard expiry.
pub const REFRESH_LEAD_TIME: Duration = Duration::from_secs(5 * 60);

/// Builds [`CliSubscriptionTokens`] from a token-endpoint response, computing
/// the absolute `expires_at` from the relative `expires_in` and carrying over
/// values the response omitted (refresh token, connection time, account id).
pub fn tokens_from_response(
    response: TokenResponse,
    previous: Option<&CliSubscriptionTokens>,
) -> CliSubscriptionTokens {
    let expires_at = response
        .expires_in
        .and_then(|secs| u64::try_from(secs).ok())
        .and_then(|secs| SystemTime::now().checked_add(Duration::from_secs(secs)));
    CliSubscriptionTokens {
        access_token: response.access_token,
        refresh_token: response
            .refresh_token
            .or_else(|| previous.and_then(|t| t.refresh_token.clone())),
        expires_at,
        connected_at: previous
            .and_then(|t| t.connected_at)
            .or_else(|| Some(SystemTime::now())),
        account_id: response
            .account_id
            .or_else(|| previous.and_then(|t| t.account_id.clone())),
    }
}

impl ApiKeyManager {
    /// Persists freshly obtained tokens for `provider` and schedules the next
    /// proactive refresh.
    pub fn store_cli_subscription_tokens(
        &mut self,
        provider: CliProvider,
        response: TokenResponse,
        ctx: &mut ModelContext<Self>,
    ) {
        apply_tokens(self, provider, response, ctx);
    }

    /// Request/proxy-time safety net: refreshes a subscription's tokens when
    /// they are near (or past) expiry.
    pub fn refresh_cli_subscription_if_needed(
        &mut self,
        provider: CliProvider,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(tokens) = self.cli_subscription(provider.id()) else {
            return;
        };
        if !tokens.needs_refresh(REFRESH_LEAD_TIME) {
            return;
        }
        let Some(refresh_token) = tokens.refresh_token.clone() else {
            return;
        };
        spawn_refresh(self, provider, refresh_token, ctx);
    }
}

fn apply_tokens(
    manager: &mut ApiKeyManager,
    provider: CliProvider,
    response: TokenResponse,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    let tokens = tokens_from_response(response, manager.cli_subscription(provider.id()));
    manager.set_cli_subscription(provider.id(), Some(tokens), ctx);
    schedule_refresh(manager, provider, ctx);
}

/// Schedules a one-shot proactive refresh [`REFRESH_LEAD_TIME`] before the
/// current token's expiry (immediately if already within that window).
fn schedule_refresh(
    manager: &mut ApiKeyManager,
    provider: CliProvider,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    let Some(tokens) = manager.cli_subscription(provider.id()) else {
        return;
    };
    let Some(refresh_token) = tokens.refresh_token.clone() else {
        return;
    };
    let Some(expires_at) = tokens.expires_at else {
        return;
    };
    let now = SystemTime::now();
    let fire_at = expires_at.checked_sub(REFRESH_LEAD_TIME).unwrap_or(now);
    let delay = fire_at.duration_since(now).unwrap_or(Duration::ZERO);

    ctx.spawn(
        async move {
            Timer::after(delay).await;
        },
        move |manager, _output, ctx| {
            let still_current = manager
                .cli_subscription(provider.id())
                .and_then(|t| t.refresh_token.as_deref())
                == Some(refresh_token.as_str());
            if still_current {
                spawn_refresh(manager, provider, refresh_token, ctx);
            }
        },
    );
}

fn spawn_refresh(
    // The refresh runs entirely on the spawned task; the callback receives its
    // own `&mut ApiKeyManager`, so the caller's reference is only needed to
    // scope the call to `ctx`.
    _manager: &mut ApiKeyManager,
    provider: CliProvider,
    refresh_token: String,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    ctx.spawn(
        async move { oauth::refresh_access_token(provider, &refresh_token).await },
        move |manager, result, ctx| match result {
            Ok(response) => {
                log::info!(
                    "Refreshed {} OAuth token (expires_in={:?})",
                    provider.display_name(),
                    response.expires_in,
                );
                apply_tokens(manager, provider, response, ctx);
            }
            Err(err) => {
                log::error!(
                    "Failed to refresh {} OAuth token: {err:#}",
                    provider.display_name()
                );
            }
        },
    );
}

#[cfg(test)]
#[path = "cli_subscription_tests.rs"]
mod tests;
