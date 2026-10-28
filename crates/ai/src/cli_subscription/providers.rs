//! Per-provider OAuth configuration for CLI-agent subscriptions.
//!
//! Each [`CliProvider`] variant carries the public OAuth client parameters used
//! by that vendor's official CLI (the same client ids the CLIs themselves ship
//! with), so Warp can run the identical browser flow and obtain a token the
//! translation proxy can use. These are public client identifiers, not
//! secrets.
//!
//! The auth mechanics differ per provider:
//!   * ChatGPT/Codex, Gemini CLI, Qwen Code: Authorization Code + PKCE with a
//!     loopback redirect (like the Grok flow).
//!   * GitHub Copilot: OAuth **device-code** flow (no loopback needed).
//!   * Claude Code: Authorization Code + PKCE against Anthropic's console.
//!
//! `client_id`s below are drawn from each vendor's open-source CLI. They can
//! change; treat this table as configuration, not a contract.

use strum_macros::EnumIter;

/// How the initial authorization is performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    /// OAuth 2.0 Authorization Code grant with PKCE and a loopback redirect.
    PkceLoopback {
        /// Fixed loopback port the provider's client_id is registered against.
        /// `0` means "any free port" (provider accepts a dynamic redirect).
        redirect_port: u16,
    },
    /// OAuth 2.0 Device Authorization Grant (RFC 8628).
    DeviceCode,
}

/// A connectable CLI-agent subscription provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum CliProvider {
    /// ChatGPT subscription via the Codex CLI OAuth client (Plus/Pro/Team).
    ChatgptCodex,
    /// Google account with a Gemini Code Assist / AI Pro entitlement, via the
    /// Gemini CLI OAuth client.
    GeminiCli,
    /// Anthropic Claude subscription (Claude Pro/Max) via the Claude Code
    /// OAuth client.
    ClaudeCode,
    /// GitHub Copilot subscription via the device-code flow used by the
    /// Copilot CLI / editor plugins.
    GithubCopilot,
    /// Alibaba Qwen Code subscription via the Qwen OAuth client.
    QwenCode,
}

/// OAuth endpoints + client parameters for a provider.
pub struct OauthConfig {
    pub client_id: &'static str,
    /// Public client secret embedded in the vendor's official CLI. Google
    /// "installed app" OAuth clients (Gemini CLI) require this in the token
    /// exchange even though it isn't confidential; PKCE-only clients (OpenAI,
    /// Anthropic, GitHub) leave it `None`.
    pub client_secret: Option<&'static str>,
    pub authorize_url: &'static str,
    pub token_url: &'static str,
    /// Device-authorization endpoint, for [`AuthKind::DeviceCode`] providers.
    pub device_code_url: Option<&'static str>,
    pub scope: &'static str,
    pub auth_kind: AuthKind,
    /// Host used in the loopback `redirect_uri` string. Most providers accept
    /// `127.0.0.1`; some (e.g. OpenAI/Codex) registered their client against
    /// `localhost` and validate the redirect URI exactly.
    pub redirect_host: &'static str,
    /// Path component of the loopback `redirect_uri` (e.g. `/callback`,
    /// `/auth/callback`). Must match what the provider's OAuth client
    /// registered, since strict servers reject any mismatch.
    pub redirect_path: &'static str,
    /// Provider-specific extra query params appended to the authorize URL.
    /// Standards-strict servers (OpenAI's Hydra) reject *unknown* params, so
    /// each provider only gets exactly the extras its own CLI sends — no
    /// cross-provider contamination.
    pub extra_authorize_params: &'static [(&'static str, &'static str)],
}

impl CliProvider {
    /// Every provider, for UI iteration without pulling in `strum`.
    pub const ALL: [CliProvider; 5] = [
        CliProvider::ChatgptCodex,
        CliProvider::GeminiCli,
        CliProvider::ClaudeCode,
        CliProvider::GithubCopilot,
        CliProvider::QwenCode,
    ];

    /// Stable slug used as the secure-storage key and custom-endpoint
    /// `provider_id`. Never change these without a migration.
    pub fn id(self) -> &'static str {
        match self {
            CliProvider::ChatgptCodex => "chatgpt_codex",
            CliProvider::GeminiCli => "gemini_cli",
            CliProvider::ClaudeCode => "claude_code",
            CliProvider::GithubCopilot => "github_copilot",
            CliProvider::QwenCode => "qwen_code",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "chatgpt_codex" => CliProvider::ChatgptCodex,
            "gemini_cli" => CliProvider::GeminiCli,
            "claude_code" => CliProvider::ClaudeCode,
            "github_copilot" => CliProvider::GithubCopilot,
            "qwen_code" => CliProvider::QwenCode,
            _ => return None,
        })
    }

    pub fn display_name(self) -> &'static str {
        match self {
            CliProvider::ChatgptCodex => "ChatGPT (Codex)",
            CliProvider::GeminiCli => "Gemini CLI",
            CliProvider::ClaudeCode => "Claude Code",
            CliProvider::GithubCopilot => "GitHub Copilot",
            CliProvider::QwenCode => "Qwen Code",
        }
    }

    /// One-line subscription requirement shown in the UI.
    pub fn requirement(self) -> &'static str {
        match self {
            CliProvider::ChatgptCodex => "Requires a ChatGPT Plus, Pro, or Team subscription.",
            CliProvider::GeminiCli => "Requires a Google account with Gemini Code Assist / AI Pro.",
            CliProvider::ClaudeCode => "Requires a Claude Pro or Max subscription.",
            CliProvider::GithubCopilot => "Requires an active GitHub Copilot subscription.",
            CliProvider::QwenCode => "Requires a Qwen (Alibaba) account with Qwen Code access.",
        }
    }

    /// Whether the translation proxy can serve this provider by pointing a
    /// custom endpoint at the local proxy. All can; Copilot is the most direct.
    pub fn auth_kind(self) -> AuthKind {
        self.oauth_config().auth_kind
    }

    /// The default upstream base URL the translation proxy targets for this
    /// provider (the proxy owns the exact request shaping).
    pub fn upstream_base_url(self) -> &'static str {
        match self {
            CliProvider::ChatgptCodex => "https://chatgpt.com/backend-api/codex",
            CliProvider::GeminiCli => "https://cloudcode-pa.googleapis.com",
            CliProvider::ClaudeCode => "https://api.anthropic.com",
            CliProvider::GithubCopilot => "https://api.githubcopilot.com",
            CliProvider::QwenCode => "https://portal.qwen.ai/v1",
        }
    }

    /// Models that can be accessed through this provider's subscription token.
    ///
    /// Each entry is `(model_slug, display_alias)`. The slug is passed through
    /// to the upstream as-is; the alias is what appears in Warp's model picker.
    /// The first entry is the recommended default.
    ///
    /// These lists are sourced from each vendor's official CLI / documentation
    /// and reflect the models available to typical paid subscribers. The exact
    /// set may vary by plan tier; models that don't exist for a user's plan
    /// will return a clear upstream error at request time.
    pub fn available_models(self) -> &'static [(&'static str, &'static str)] {
        match self {
            CliProvider::ChatgptCodex => &[
                ("o3", "o3 (ChatGPT)"),
                ("o4-mini", "o4-mini (ChatGPT)"),
                ("gpt-4.1", "GPT-4.1 (ChatGPT)"),
                ("gpt-4.1-mini", "GPT-4.1 mini (ChatGPT)"),
                ("gpt-4.1-nano", "GPT-4.1 nano (ChatGPT)"),
                ("gpt-4o", "GPT-4o (ChatGPT)"),
                ("gpt-4o-mini", "GPT-4o mini (ChatGPT)"),
            ],
            CliProvider::GeminiCli => &[
                ("gemini-2.5-pro", "Gemini 2.5 Pro (CLI)"),
                ("gemini-2.5-flash", "Gemini 2.5 Flash (CLI)"),
                ("gemini-2.0-flash", "Gemini 2.0 Flash (CLI)"),
                ("gemini-2.0-flash-lite", "Gemini 2.0 Flash Lite (CLI)"),
            ],
            CliProvider::ClaudeCode => &[
                ("claude-sonnet-4-20250514", "Claude Sonnet 4 (Claude Code)"),
                (
                    "claude-3-5-sonnet-20241022",
                    "Claude 3.5 Sonnet (Claude Code)",
                ),
                (
                    "claude-3-5-haiku-20241022",
                    "Claude 3.5 Haiku (Claude Code)",
                ),
            ],
            CliProvider::GithubCopilot => &[
                ("gpt-4.1", "GPT-4.1 (Copilot)"),
                ("gpt-4o", "GPT-4o (Copilot)"),
                ("claude-3.5-sonnet", "Claude 3.5 Sonnet (Copilot)"),
                ("o3-mini", "o3-mini (Copilot)"),
            ],
            CliProvider::QwenCode => &[
                ("qwen3-coder-plus", "Qwen3 Coder Plus (Qwen Code)"),
                ("qwen3-coder", "Qwen3 Coder (Qwen Code)"),
                ("qwen-plus", "Qwen Plus (Qwen Code)"),
            ],
        }
    }

    /// Public OAuth client parameters for this provider, sourced from the
    /// vendor's official CLI.
    pub fn oauth_config(self) -> OauthConfig {
        match self {
            CliProvider::ChatgptCodex => OauthConfig {
                client_id: "app_EMoamEEZ73f0CkXaXp7hrann",
                client_secret: None,
                authorize_url: "https://auth.openai.com/oauth/authorize",
                token_url: "https://auth.openai.com/oauth/token",
                device_code_url: None,
                scope: "openid profile email offline_access",
                auth_kind: AuthKind::PkceLoopback {
                    redirect_port: 1455,
                },
                // OpenAI's Codex client registered `http://localhost:1455/auth/callback`
                // exactly, and its Hydra server rejects unknown/mismatched params.
                redirect_host: "localhost",
                redirect_path: "/auth/callback",
                extra_authorize_params: &[
                    ("id_token_add_organizations", "true"),
                    ("codex_cli_simplified_flow", "true"),
                    ("originator", "codex_cli_rs"),
                ],
            },
            CliProvider::GeminiCli => OauthConfig {
                // Gemini CLI's public OAuth client (a Google "Desktop app"
                // client, so any loopback port is accepted).
                client_id:
                    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com",
                client_secret: Some("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"),
                authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
                token_url: "https://oauth2.googleapis.com/token",
                device_code_url: None,
                scope: "https://www.googleapis.com/auth/cloud-platform \
                        https://www.googleapis.com/auth/userinfo.email \
                        https://www.googleapis.com/auth/userinfo.profile",
                auth_kind: AuthKind::PkceLoopback { redirect_port: 0 },
                redirect_host: "127.0.0.1",
                redirect_path: "/oauth2callback",
                // Google needs these to return a refresh token every time.
                extra_authorize_params: &[("access_type", "offline"), ("prompt", "consent")],
            },
            CliProvider::ClaudeCode => OauthConfig {
                client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
                client_secret: None,
                authorize_url: "https://claude.ai/oauth/authorize",
                token_url: "https://console.anthropic.com/v1/oauth/token",
                device_code_url: None,
                scope: "org:create_api_key user:profile user:inference",
                auth_kind: AuthKind::PkceLoopback {
                    redirect_port: 54545,
                },
                redirect_host: "localhost",
                redirect_path: "/callback",
                // Anthropic's console flow expects `code=true`.
                extra_authorize_params: &[("code", "true")],
            },
            CliProvider::GithubCopilot => OauthConfig {
                // GitHub's public "GitHub for VS Code" / Copilot client id.
                client_id: "Iv1.b507a08c87ecfe98",
                client_secret: None,
                authorize_url: "https://github.com/login/oauth/authorize",
                token_url: "https://github.com/login/oauth/access_token",
                device_code_url: Some("https://github.com/login/device/code"),
                scope: "read:user",
                auth_kind: AuthKind::DeviceCode,
                // Unused for the device-code flow, but kept well-formed.
                redirect_host: "127.0.0.1",
                redirect_path: "/callback",
                extra_authorize_params: &[],
            },
            CliProvider::QwenCode => OauthConfig {
                client_id: "f0304373b74a44d2b584a3fb70ca9e56",
                client_secret: None,
                authorize_url: "https://chat.qwen.ai/api/v1/oauth2/device/code",
                token_url: "https://chat.qwen.ai/api/v1/oauth2/token",
                device_code_url: Some("https://chat.qwen.ai/api/v1/oauth2/device/code"),
                scope: "openid profile email model.completion",
                auth_kind: AuthKind::DeviceCode,
                redirect_host: "127.0.0.1",
                redirect_path: "/callback",
                extra_authorize_params: &[],
            },
        }
    }
}

#[cfg(test)]
#[path = "providers_tests.rs"]
mod tests;
