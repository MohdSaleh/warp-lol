//! A built-in catalog of well-known LLM API providers that expose an
//! OpenAI-compatible Chat Completions endpoint.
//!
//! Warp's BYO "custom endpoints" ([`crate::api_keys::CustomEndpoint`]) already
//! support any OpenAI-compatible endpoint, but require the user to hand-enter
//! the base URL and model slugs. This catalog pre-fills those details for the
//! most common providers so connecting a new provider is just "pick provider,
//! paste key". Multiple providers can be configured simultaneously — each
//! catalog entry becomes its own custom endpoint, and every model from every
//! endpoint shows up in the model picker at the same time.
//!
//! The catalog is intentionally data-only (no networking) so it compiles on
//! every target, including wasm. Model lists are *suggestions* seeded into the
//! endpoint editor; providers add and retire models frequently, so users can
//! freely edit the rows after selecting a preset.

/// How the endpoint's base URL is obtained for a catalog provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderUrlKind {
    /// The URL is fixed and known ahead of time; the user only pastes a key.
    Fixed,
    /// The preset URL contains a placeholder segment (e.g. an Azure resource
    /// name) the user must edit before saving.
    Template,
    /// The provider is self-hosted (Ollama, LM Studio, LiteLLM, ...) so the
    /// user must supply the full URL. In Local Mode, `http://localhost` is permitted.
    SelfHosted,
}

/// One predefined provider in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderPreset {
    /// Stable identifier persisted on endpoints created from this preset
    /// (see [`crate::api_keys::CustomEndpoint::provider_id`]).
    pub id: &'static str,
    /// Human-readable name shown in the picker ("Groq", "Mistral", ...).
    pub display_name: &'static str,
    /// OpenAI-compatible Chat Completions base URL (without the trailing
    /// `/chat/completions`). Empty for [`ProviderUrlKind::SelfHosted`].
    pub base_url: &'static str,
    pub url_kind: ProviderUrlKind,
    /// Placeholder text for the API key input (e.g. "gsk_...").
    pub key_placeholder: &'static str,
    /// Where the user can create/manage API keys for this provider.
    pub key_url: &'static str,
    /// Suggested model slugs seeded into the endpoint editor. `(slug, alias)`.
    pub suggested_models: &'static [(&'static str, &'static str)],
    /// Extra one-line note surfaced in the UI (caveats, requirements).
    pub note: Option<&'static str>,
}

/// The full provider catalog, in the order presets are shown in the UI.
///
/// Only providers with an OpenAI-compatible Chat Completions API belong here;
/// subscription/OAuth-based providers (ChatGPT/Codex, Gemini CLI, Claude
/// Code, GitHub Copilot, ...) are handled by `crate::cli_subscription`
/// instead. OpenAI, Anthropic, Google, and OpenRouter are also not listed —
/// they already have first-class API key fields in settings.
pub const PROVIDER_CATALOG: &[ProviderPreset] = &[
    ProviderPreset {
        id: "groq",
        display_name: "Groq",
        base_url: "https://api.groq.com/openai/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "gsk_...",
        key_url: "https://console.groq.com/keys",
        suggested_models: &[
            ("llama-3.3-70b-versatile", "Llama 3.3 70B"),
            ("moonshotai/kimi-k2-instruct", "Kimi K2 (Groq)"),
        ],
        note: None,
    },
    ProviderPreset {
        id: "mistral",
        display_name: "Mistral",
        base_url: "https://api.mistral.ai/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "API key from La Plateforme",
        key_url: "https://console.mistral.ai/api-keys",
        suggested_models: &[
            ("mistral-large-latest", "Mistral Large"),
            ("codestral-latest", "Codestral"),
            ("devstral-medium-latest", "Devstral"),
        ],
        note: None,
    },
    ProviderPreset {
        id: "deepseek",
        display_name: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "sk-...",
        key_url: "https://platform.deepseek.com/api_keys",
        suggested_models: &[
            ("deepseek-chat", "DeepSeek V3"),
            ("deepseek-reasoner", "DeepSeek R1"),
        ],
        note: None,
    },
    ProviderPreset {
        id: "xai_api",
        display_name: "xAI (API key)",
        base_url: "https://api.x.ai/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "xai-...",
        key_url: "https://console.x.ai",
        suggested_models: &[("grok-4", "Grok 4"), ("grok-code-fast-1", "Grok Code")],
        note: Some(
            "Pay-as-you-go xAI API. To use a SuperGrok subscription instead, use \
             \"Connect\" in the subscription section.",
        ),
    },
    ProviderPreset {
        id: "together",
        display_name: "Together AI",
        base_url: "https://api.together.xyz/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "API key",
        key_url: "https://api.together.ai/settings/api-keys",
        suggested_models: &[
            ("meta-llama/Llama-3.3-70B-Instruct-Turbo", "Llama 3.3 70B"),
            ("Qwen/Qwen2.5-Coder-32B-Instruct", "Qwen Coder 32B"),
        ],
        note: None,
    },
    ProviderPreset {
        id: "fireworks",
        display_name: "Fireworks AI",
        base_url: "https://api.fireworks.ai/inference/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "fw_...",
        key_url: "https://app.fireworks.ai/settings/users/api-keys",
        suggested_models: &[(
            "accounts/fireworks/models/deepseek-v3",
            "DeepSeek V3 (Fireworks)",
        )],
        note: None,
    },
    ProviderPreset {
        id: "cerebras",
        display_name: "Cerebras",
        base_url: "https://api.cerebras.ai/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "csk-...",
        key_url: "https://cloud.cerebras.ai",
        suggested_models: &[
            ("llama-3.3-70b", "Llama 3.3 70B (Cerebras)"),
            ("qwen-3-coder-480b", "Qwen3 Coder (Cerebras)"),
        ],
        note: None,
    },
    ProviderPreset {
        id: "perplexity",
        display_name: "Perplexity",
        base_url: "https://api.perplexity.ai",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "pplx-...",
        key_url: "https://www.perplexity.ai/settings/api",
        suggested_models: &[("sonar-pro", "Sonar Pro"), ("sonar", "Sonar")],
        note: None,
    },
    ProviderPreset {
        id: "moonshot",
        display_name: "Moonshot (Kimi)",
        base_url: "https://api.moonshot.ai/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "sk-...",
        key_url: "https://platform.moonshot.ai/console/api-keys",
        suggested_models: &[("kimi-k2-0711-preview", "Kimi K2")],
        note: None,
    },
    ProviderPreset {
        id: "zhipu",
        display_name: "Z.ai (GLM)",
        base_url: "https://api.z.ai/api/paas/v4",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "API key",
        key_url: "https://z.ai/manage-apikey/apikey-list",
        suggested_models: &[("glm-4.5", "GLM 4.5"), ("glm-4.5-air", "GLM 4.5 Air")],
        note: None,
    },
    ProviderPreset {
        id: "qwen_dashscope",
        display_name: "Alibaba Qwen (DashScope)",
        base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "sk-...",
        key_url: "https://bailian.console.alibabacloud.com",
        suggested_models: &[
            ("qwen3-coder-plus", "Qwen3 Coder Plus"),
            ("qwen-max", "Qwen Max"),
        ],
        note: Some("International endpoint. Mainland China accounts should edit the URL to dashscope.aliyuncs.com."),
    },
    ProviderPreset {
        id: "minimax",
        display_name: "MiniMax",
        base_url: "https://api.minimax.io/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "API key",
        key_url: "https://platform.minimax.io/user-center/basic-information/interface-key",
        suggested_models: &[("MiniMax-M1", "MiniMax M1")],
        note: None,
    },
    ProviderPreset {
        id: "huggingface",
        display_name: "Hugging Face (Inference Providers)",
        base_url: "https://router.huggingface.co/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "hf_...",
        key_url: "https://huggingface.co/settings/tokens",
        suggested_models: &[
            ("deepseek-ai/DeepSeek-V3", "DeepSeek V3 (HF)"),
            ("Qwen/Qwen3-Coder-480B-A35B-Instruct", "Qwen3 Coder (HF)"),
        ],
        note: Some("Uses your Hugging Face access token (same account as `hf auth login`)."),
    },
    ProviderPreset {
        id: "nvidia_nim",
        display_name: "NVIDIA NIM",
        base_url: "https://integrate.api.nvidia.com/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "nvapi-...",
        key_url: "https://build.nvidia.com",
        suggested_models: &[("meta/llama-3.3-70b-instruct", "Llama 3.3 70B (NIM)")],
        note: None,
    },
    ProviderPreset {
        id: "sambanova",
        display_name: "SambaNova",
        base_url: "https://api.sambanova.ai/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "API key",
        key_url: "https://cloud.sambanova.ai/apis",
        suggested_models: &[("Meta-Llama-3.3-70B-Instruct", "Llama 3.3 70B (SambaNova)")],
        note: None,
    },
    ProviderPreset {
        id: "github_models",
        display_name: "GitHub Models",
        base_url: "https://models.github.ai/inference",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "github_pat_... (fine-grained PAT with models:read)",
        key_url: "https://github.com/settings/personal-access-tokens",
        suggested_models: &[("openai/gpt-4.1", "GPT-4.1 (GitHub Models)")],
        note: Some("Free tier is heavily rate-limited; intended for evaluation."),
    },
    ProviderPreset {
        id: "vercel_gateway",
        display_name: "Vercel AI Gateway",
        base_url: "https://ai-gateway.vercel.sh/v1",
        url_kind: ProviderUrlKind::Fixed,
        key_placeholder: "vck_...",
        key_url: "https://vercel.com/d?to=%2F%5Bteam%5D%2F%7E%2Fai%2Fapi-keys",
        suggested_models: &[("anthropic/claude-sonnet-4", "Claude Sonnet (Gateway)")],
        note: None,
    },
    ProviderPreset {
        id: "azure_openai",
        display_name: "Azure OpenAI",
        base_url: "https://YOUR-RESOURCE.openai.azure.com/openai/v1",
        url_kind: ProviderUrlKind::Template,
        key_placeholder: "Azure API key",
        key_url: "https://portal.azure.com",
        suggested_models: &[("gpt-4.1", "GPT-4.1 (Azure)")],
        note: Some("Replace YOUR-RESOURCE with your Azure OpenAI resource name; model names are your deployment names."),
    },
    ProviderPreset {
        id: "ollama",
        display_name: "Ollama (self-hosted)",
        base_url: "",
        url_kind: ProviderUrlKind::SelfHosted,
        key_placeholder: "ollama (any non-empty value)",
        key_url: "https://ollama.com/download",
        suggested_models: &[("qwen2.5-coder:32b", "Qwen Coder (Ollama)")],
        note: Some(
            "Ollama's OpenAI-compatible API is at /v1. E.g., http://localhost:11434/v1 for Local Mode.",
        ),
    },
    ProviderPreset {
        id: "lm_studio",
        display_name: "LM Studio (self-hosted)",
        base_url: "",
        url_kind: ProviderUrlKind::SelfHosted,
        key_placeholder: "lm-studio (any non-empty value)",
        key_url: "https://lmstudio.ai",
        suggested_models: &[],
        note: Some(
            "LM Studio's local server speaks the OpenAI API at /v1. E.g., http://localhost:1234/v1 for Local Mode.",
        ),
    },
    ProviderPreset {
        id: "litellm",
        display_name: "LiteLLM gateway (self-hosted)",
        base_url: "",
        url_kind: ProviderUrlKind::SelfHosted,
        key_placeholder: "sk-... (LiteLLM virtual key)",
        key_url: "https://docs.litellm.ai",
        suggested_models: &[],
        note: Some("Point at your LiteLLM proxy's public HTTPS URL."),
    },
];

/// Looks up a preset by its stable [`ProviderPreset::id`].
pub fn preset_by_id(id: &str) -> Option<&'static ProviderPreset> {
    PROVIDER_CATALOG.iter().find(|preset| preset.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = PROVIDER_CATALOG.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        let len_before = ids.len();
        ids.dedup();
        assert_eq!(len_before, ids.len(), "duplicate provider preset ids");
    }

    #[test]
    fn fixed_urls_are_https() {
        for preset in PROVIDER_CATALOG {
            match preset.url_kind {
                ProviderUrlKind::Fixed | ProviderUrlKind::Template => {
                    assert!(
                        preset.base_url.starts_with("https://"),
                        "{} must have an https base URL",
                        preset.id
                    );
                }
                ProviderUrlKind::SelfHosted => {
                    assert!(preset.base_url.is_empty());
                }
            }
        }
    }

    #[test]
    fn lookup_by_id() {
        assert_eq!(preset_by_id("groq").unwrap().display_name, "Groq");
        assert!(preset_by_id("nope").is_none());
    }
}
