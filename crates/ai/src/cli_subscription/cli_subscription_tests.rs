use std::time::{Duration, SystemTime};

use super::oauth::TokenResponse;
use super::providers::CliProvider;
use super::tokens_from_response;
use crate::api_keys::CliSubscriptionTokens;

#[test]
fn tokens_from_response_computes_expiry_and_carries_over() {
    let previous = CliSubscriptionTokens {
        access_token: "old".into(),
        refresh_token: Some("refresh-1".into()),
        expires_at: None,
        connected_at: Some(SystemTime::UNIX_EPOCH),
        account_id: Some("acct-1".into()),
    };
    let response = TokenResponse {
        access_token: "new".into(),
        refresh_token: None,
        expires_in: Some(3600),
        account_id: None,
        ..Default::default()
    };
    let tokens = tokens_from_response(response, Some(&previous));
    assert_eq!(tokens.access_token, "new");
    // Refresh token, connection time, and account id carry over when omitted.
    assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
    assert_eq!(tokens.connected_at, Some(SystemTime::UNIX_EPOCH));
    assert_eq!(tokens.account_id.as_deref(), Some("acct-1"));
    assert!(tokens.expires_at.is_some());
    assert!(!tokens.needs_refresh(Duration::from_secs(60)));
}

#[test]
fn fresh_connect_initializes_connected_at() {
    let response = TokenResponse {
        access_token: "tok".into(),
        refresh_token: Some("r".into()),
        expires_in: Some(10),
        account_id: Some("a".into()),
        ..Default::default()
    };
    let tokens = tokens_from_response(response, None);
    assert!(tokens.connected_at.is_some());
    // Expires in 10s, so it needs refresh within a 60s lead time.
    assert!(tokens.needs_refresh(Duration::from_secs(60)));
}

#[test]
fn all_providers_have_upstream_urls() {
    use strum::IntoEnumIterator;
    for provider in CliProvider::iter() {
        assert!(provider.upstream_base_url().starts_with("https://"));
        assert!(!provider.requirement().is_empty());
    }
}
