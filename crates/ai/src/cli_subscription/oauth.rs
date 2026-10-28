//! Provider-agnostic OAuth mechanics for CLI-subscription connect flows.
//!
//! Two grant types are supported, selected per provider by
//! [`super::providers::AuthKind`]:
//!
//!   * [`PkceLoopbackAttempt`] — Authorization Code + PKCE with a loopback
//!     redirect, generalizing the Grok flow ([`crate::grok_subscription`]) to
//!     any provider's endpoints/client id.
//!   * [`DeviceCodeAttempt`] — OAuth 2.0 Device Authorization Grant (RFC 8628),
//!     used by GitHub Copilot and Qwen Code, which need no loopback server.
//!
//! This module owns only network/protocol concerns. Token persistence and
//! refresh scheduling live in the parent [`super`] module and
//! [`crate::api_keys::ApiKeyManager`].

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;

use anyhow::{bail, Context as _};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use instant::Instant;
use rand::RngCore as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::providers::{AuthKind, CliProvider, OauthConfig};

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Normalized token-endpoint response across all providers. Unknown fields are
/// ignored.
#[derive(Debug, Deserialize, Default)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
    /// Provider-specific account/project id when the token endpoint returns
    /// one (e.g. ChatGPT account id embedded in the id_token, or a Google
    /// project). Populated by provider-specific post-processing.
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
}

// ── Authorization Code + PKCE (loopback) ────────────────────────────────────

/// One in-flight loopback PKCE attempt for a given provider.
pub struct PkceLoopbackAttempt {
    provider: CliProvider,
    listener: TcpListener,
    redirect_uri: String,
    /// The callback path we listen for (provider-specific, e.g.
    /// `/auth/callback`).
    redirect_path: &'static str,
    pkce: PkceParams,
}

impl PkceLoopbackAttempt {
    /// Binds the loopback callback server (fixed port if the provider requires
    /// one, else an OS-assigned port) and generates fresh PKCE/CSRF secrets.
    pub fn start(provider: CliProvider) -> anyhow::Result<Self> {
        let AuthKind::PkceLoopback { redirect_port } = provider.auth_kind() else {
            bail!(
                "{} does not use the loopback PKCE flow",
                provider.display_name()
            );
        };
        let cfg = provider.oauth_config();
        let listener = bind_listener(redirect_port).with_context(|| {
            format!(
                "couldn't bind the OAuth callback server for {}. Another login may be in \
                 progress, or the provider's CLI is using the port.",
                provider.display_name()
            )
        })?;
        let port = listener
            .local_addr()
            .context("failed to read the OAuth callback server address")?
            .port();
        // The redirect_uri STRING must match what the provider's client
        // registered (host + path); we still bind on 127.0.0.1 locally, which
        // `localhost` resolves to.
        Ok(Self {
            provider,
            listener,
            redirect_uri: format!("http://{}:{port}{}", cfg.redirect_host, cfg.redirect_path),
            redirect_path: cfg.redirect_path,
            pkce: PkceParams::generate(),
        })
    }

    /// The authorization URL to open in the user's browser.
    pub fn authorize_url(&self) -> String {
        let cfg = self.provider.oauth_config();
        let mut params: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", cfg.client_id),
            ("redirect_uri", &self.redirect_uri),
            ("scope", cfg.scope),
            ("code_challenge", &self.pkce.challenge),
            ("code_challenge_method", "S256"),
            ("state", &self.pkce.state),
        ];
        // Only the provider's own extra params — strict servers (OpenAI Hydra)
        // reject unknown ones.
        params.extend_from_slice(cfg.extra_authorize_params);
        let query =
            serde_urlencoded::to_string(&params).expect("OAuth params are always serializable");
        format!("{}?{query}", cfg.authorize_url)
    }

    /// Waits for the loopback callback, validates CSRF state, and exchanges the
    /// code for tokens.
    pub async fn finish(self) -> anyhow::Result<TokenResponse> {
        let provider = self.provider;
        let (tx, rx) = async_channel::bounded(1);
        let listener = self.listener;
        let redirect_path = self.redirect_path;
        let display_name = provider.display_name();
        std::thread::Builder::new()
            .name("cli-oauth-callback".to_owned())
            .spawn(move || {
                let _ = warpui_core::r#async::block_on(tx.send(wait_for_callback(
                    &listener,
                    redirect_path,
                    CALLBACK_TIMEOUT,
                )));
            })
            .with_context(|| format!("failed to spawn the {display_name} OAuth callback thread"))?;

        let callback = rx
            .recv()
            .await
            .with_context(|| format!("the {display_name} OAuth callback server stopped"))??;

        if callback.state != self.pkce.state {
            bail!("the authorization response state did not match — aborting to prevent CSRF");
        }
        exchange_code_for_tokens(
            provider,
            &callback.code,
            &self.redirect_uri,
            &self.pkce.verifier,
        )
        .await
    }
}

struct PkceParams {
    verifier: String,
    challenge: String,
    state: String,
}

impl PkceParams {
    fn generate() -> Self {
        let verifier = random_url_safe_token();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
            state: random_url_safe_token(),
        }
    }
}

fn random_url_safe_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

struct CallbackData {
    code: String,
    state: String,
}

fn bind_listener(port: u16) -> anyhow::Result<TcpListener> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    listener
        .set_nonblocking(true)
        .context("failed to set the OAuth callback listener to non-blocking mode")?;
    Ok(listener)
}

fn wait_for_callback(
    listener: &TcpListener,
    redirect_path: &str,
    timeout: Duration,
) -> anyhow::Result<CallbackData> {
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            bail!("timed out waiting for the authorization callback");
        }
        match listener.accept() {
            Ok((stream, _)) => match handle_callback_connection(stream, redirect_path)? {
                Some(data) => return Ok(data),
                None => continue,
            },
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => return Err(anyhow::Error::new(e).context("OAuth callback accept failed")),
        }
    }
}

fn handle_callback_connection(
    mut stream: TcpStream,
    redirect_path: &str,
) -> anyhow::Result<Option<CallbackData>> {
    stream.set_nonblocking(false).ok();
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();

    let mut buf = [0u8; 8192];
    let n = stream
        .read(&mut buf)
        .context("failed to read the OAuth callback request")?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let path = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .nth(1)
        .unwrap_or_default();

    let Some(query) = path
        .strip_prefix(redirect_path)
        .and_then(|rest| rest.strip_prefix('?'))
    else {
        write_response(&mut stream, "404 Not Found", "Not found.");
        return Ok(None);
    };

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let pairs: Vec<(String, String)> = serde_urlencoded::from_str(query).unwrap_or_default();
    for (key, value) in pairs {
        match key.as_str() {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            _ => {}
        }
    }

    if let Some(error) = error {
        write_response(&mut stream, "400 Bad Request", FAILURE_HTML);
        bail!("authorization was denied or failed: {error}");
    }
    let (Some(code), Some(state)) = (code, state) else {
        write_response(&mut stream, "400 Bad Request", FAILURE_HTML);
        bail!("the authorization callback was missing the code or state parameter");
    };
    write_response(&mut stream, "200 OK", SUCCESS_HTML);
    Ok(Some(CallbackData { code, state }))
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

async fn exchange_code_for_tokens(
    provider: CliProvider,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> anyhow::Result<TokenResponse> {
    let cfg = provider.oauth_config();
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", cfg.client_id),
        ("code_verifier", verifier),
    ];
    // Google installed-app clients require the (public) client secret in the
    // exchange; omitting it yields "client_secret is missing".
    if let Some(secret) = cfg.client_secret {
        form.push(("client_secret", secret));
    }
    post_token_request(provider, &cfg, &form).await
}

// ── Device Authorization Grant (RFC 8628) ───────────────────────────────────

/// A started device-code flow: show `verification_uri` + `user_code` to the
/// user, then poll [`DeviceCodeAttempt::poll`] until they approve.
#[derive(Debug, Deserialize)]
pub struct DeviceCodeAttempt {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    #[serde(default = "default_interval")]
    pub interval: i64,
    #[serde(skip)]
    provider: Option<CliProvider>,
}

fn default_interval() -> i64 {
    5
}

impl DeviceCodeAttempt {
    /// Requests a device + user code from the provider's device-authorization
    /// endpoint.
    pub async fn start(provider: CliProvider) -> anyhow::Result<Self> {
        let cfg = provider.oauth_config();
        let AuthKind::DeviceCode = cfg.auth_kind else {
            bail!(
                "{} does not use the device-code flow",
                provider.display_name()
            );
        };
        let url = cfg
            .device_code_url
            .context("provider is missing a device-code endpoint")?;
        let form: [(&str, &str); 2] = [("client_id", cfg.client_id), ("scope", cfg.scope)];
        let response = http_client::Client::new()
            .post(url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .context("failed to request a device code")?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("device code request failed ({status}): {body}");
        }
        let mut attempt: DeviceCodeAttempt = response
            .json()
            .await
            .context("failed to parse the device code response")?;
        attempt.provider = Some(provider);
        Ok(attempt)
    }

    /// Polls the token endpoint once. Returns `Ok(Some(tokens))` when the user
    /// has approved, `Ok(None)` while still pending (caller should wait
    /// `interval` seconds and retry), or `Err` on a terminal failure.
    pub async fn poll(&self) -> anyhow::Result<Option<TokenResponse>> {
        let provider = self
            .provider
            .context("device-code attempt is missing its provider")?;
        let cfg = provider.oauth_config();
        let form: [(&str, &str); 3] = [
            ("client_id", cfg.client_id),
            ("device_code", &self.device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];
        let response = http_client::Client::new()
            .post(cfg.token_url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .context("failed to poll the device token endpoint")?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if let Ok(err) = serde_json::from_str::<DevicePollError>(&body) {
            match err.error.as_str() {
                "authorization_pending" | "slow_down" => return Ok(None),
                other => bail!("device authorization failed: {other}"),
            }
        }
        if !status.is_success() {
            bail!("device token poll failed ({status}): {body}");
        }
        let tokens: TokenResponse =
            serde_json::from_str(&body).context("failed to parse the device token response")?;
        Ok(Some(tokens))
    }
}

#[derive(Debug, Deserialize)]
struct DevicePollError {
    error: String,
}

// ── Refresh (shared) ────────────────────────────────────────────────────────

/// Exchanges a refresh token for a fresh access token.
pub async fn refresh_access_token(
    provider: CliProvider,
    refresh_token: &str,
) -> anyhow::Result<TokenResponse> {
    let cfg = provider.oauth_config();
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", cfg.client_id),
    ];
    if let Some(secret) = cfg.client_secret {
        form.push(("client_secret", secret));
    }
    post_token_request(provider, &cfg, &form).await
}

async fn post_token_request<T: serde::Serialize + ?Sized>(
    provider: CliProvider,
    cfg: &OauthConfig,
    form: &T,
) -> anyhow::Result<TokenResponse> {
    let response = http_client::Client::new()
        .post(cfg.token_url)
        .header("Accept", "application/json")
        .form(form)
        .send()
        .await
        .context("failed to send the token request")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("token request failed ({status}): {body}");
    }
    let token_resp = response
        .json::<TokenResponse>()
        .await
        .context("failed to parse the token response")?;

    post_process_token_response(provider, token_resp).await
}

async fn post_process_token_response(
    provider: CliProvider,
    mut response: TokenResponse,
) -> anyhow::Result<TokenResponse> {
    match provider {
        CliProvider::ChatgptCodex => {
            if let Some(id_token) = &response.id_token {
                log::info!(
                    "ChatGPT Codex: id_token present ({} bytes), extracting account_id",
                    id_token.len()
                );
                match extract_chatgpt_account_id(id_token) {
                    Some(account_id) => {
                        log::info!("ChatGPT Codex: extracted account_id successfully");
                        response.account_id = Some(account_id);
                    }
                    None => {
                        log::warn!(
                            "ChatGPT Codex: id_token was present but no account_id could be \
                             extracted from the JWT claims"
                        );
                    }
                }
            } else {
                log::info!(
                    "ChatGPT Codex: no id_token in token response (normal for refresh grants)"
                );
            }
        }
        CliProvider::GeminiCli => {
            // The official Gemini CLI reads the project from environment
            // variables first (GOOGLE_CLOUD_PROJECT, GOOGLE_CLOUD_PROJECT_ID,
            // GOOGLE_CLOUD_QUOTA_PROJECT). Only fall back to the Cloud Resource
            // Manager API if none are set.
            let env_project = std::env::var("GOOGLE_CLOUD_PROJECT")
                .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT_ID"))
                .or_else(|_| std::env::var("GOOGLE_CLOUD_QUOTA_PROJECT"))
                .ok()
                .filter(|s| !s.is_empty());

            if let Some(project) = env_project {
                log::info!("Gemini CLI: using project from environment variable: {project}");
                response.account_id = Some(project);
            } else {
                log::info!(
                    "Gemini CLI: no GOOGLE_CLOUD_PROJECT env var set, \
                     querying Cloud Resource Manager API"
                );
                match fetch_first_gcp_project(&response.access_token).await {
                    Ok(project_id) => {
                        log::info!("Gemini CLI: resolved project_id from GCP API: {project_id}");
                        response.account_id = Some(project_id);
                    }
                    Err(err) => {
                        // Personal Google accounts may not have any GCP
                        // projects. The cloudcode-pa endpoint can work without
                        // a project for personal accounts, so this is
                        // non-fatal.
                        log::warn!(
                            "Gemini CLI: could not resolve a GCP project \
                             (personal accounts may not need one): {err:#}"
                        );
                    }
                }
            }
        }
        _ => {}
    }
    Ok(response)
}

fn extract_chatgpt_account_id(id_token: &str) -> Option<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 {
        log::warn!("ChatGPT id_token has fewer than 2 dot-separated segments");
        return None;
    }
    let payload_b64 = parts[1];

    // JWTs use URL-safe base64 without padding. Some issuers occasionally
    // include standard base64 or padding, so try URL_SAFE_NO_PAD first, then
    // fall back to URL_SAFE (with padding) and finally STANDARD_NO_PAD.
    let decoded = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload_b64))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64));

    let decoded = match decoded {
        Ok(bytes) => bytes,
        Err(err) => {
            log::warn!("ChatGPT id_token: base64 decode of payload failed: {err}");
            return None;
        }
    };

    let json: serde_json::Value = match serde_json::from_slice(&decoded) {
        Ok(v) => v,
        Err(err) => {
            log::warn!("ChatGPT id_token: JSON parse of payload failed: {err}");
            return None;
        }
    };

    // 1. Top-level claim (newer tokens).
    if let Some(id) = json.get("chatgpt_account_id").and_then(|v| v.as_str()) {
        log::debug!("ChatGPT: found chatgpt_account_id at top level");
        return Some(id.to_string());
    }
    // 2. Namespaced under https://api.openai.com/auth (standard Codex CLI
    //    claim structure, as seen in openai/codex token_data.rs).
    if let Some(auth) = json.get("https://api.openai.com/auth") {
        if let Some(id) = auth.get("chatgpt_account_id").and_then(|v| v.as_str()) {
            log::debug!("ChatGPT: found chatgpt_account_id under /auth namespace");
            return Some(id.to_string());
        }
    }
    // 3. Organization id from the profile namespace.
    if let Some(profile) = json.get("https://api.openai.com/profile") {
        if let Some(orgs) = profile.get("organizations").and_then(|v| v.as_array()) {
            if let Some(first_org) = orgs.first() {
                if let Some(id) = first_org.get("id").and_then(|v| v.as_str()) {
                    log::debug!("ChatGPT: using first organization id as account_id");
                    return Some(id.to_string());
                }
            }
        }
    }
    // 4. Last resort: the JWT subject.
    if let Some(sub) = json.get("sub").and_then(|v| v.as_str()) {
        log::debug!("ChatGPT: falling back to 'sub' claim as account_id");
        return Some(sub.to_string());
    }

    log::warn!("ChatGPT id_token: no account_id, organization, or sub claim found in JWT payload");
    None
}

async fn fetch_first_gcp_project(access_token: &str) -> anyhow::Result<String> {
    let client = http_client::Client::new();
    let res = client
        .get("https://cloudresourcemanager.googleapis.com/v1/projects")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        bail!("GCP Resource Manager API returned status {status}: {body}");
    }

    let json: serde_json::Value = res.json().await?;
    let projects = json
        .get("projects")
        .and_then(|p| p.as_array())
        .context("no projects array returned from GCP API")?;

    log::info!("Gemini CLI: GCP API returned {} project(s)", projects.len());

    for proj in projects {
        if let Some(state) = proj.get("lifecycleState").and_then(|v| v.as_str()) {
            if state == "ACTIVE" {
                if let Some(project_id) = proj.get("projectId").and_then(|v| v.as_str()) {
                    return Ok(project_id.to_string());
                }
            }
        }
    }

    bail!(
        "no active GCP projects found among {} returned",
        projects.len()
    )
}

const SUCCESS_HTML: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Warp — connected</title></head>\
<body style=\"font-family:system-ui,-apple-system,sans-serif;text-align:center;padding:3rem\">\
<h1>Connected</h1><p>You can close this window and return to Warp.</p></body></html>";

const FAILURE_HTML: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Warp — authorization failed</title></head>\
<body style=\"font-family:system-ui,-apple-system,sans-serif;text-align:center;padding:3rem\">\
<h1>Authorization failed</h1><p>Return to Warp and try again.</p></body></html>";
