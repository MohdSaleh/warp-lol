use strum::IntoEnumIterator;

use super::*;

#[test]
fn ids_roundtrip_and_are_unique() {
    let mut ids = Vec::new();
    for provider in CliProvider::iter() {
        let id = provider.id();
        assert_eq!(CliProvider::from_id(id), Some(provider));
        ids.push(id);
    }
    let len = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(len, ids.len(), "duplicate provider ids");
}

#[test]
fn configs_are_populated() {
    for provider in CliProvider::iter() {
        let cfg = provider.oauth_config();
        assert!(!cfg.client_id.is_empty());
        assert!(cfg.token_url.starts_with("https://"));
        if let AuthKind::DeviceCode = cfg.auth_kind {
            assert!(cfg.device_code_url.is_some());
        }
    }
}
