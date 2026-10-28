use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    LocalOnly,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkEgressPolicy {
    DenyOfficialWarpBackend,
    AllowOfficialWarpBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryEgressPolicy {
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedEndpointKind {
    Loopback,
    UserConfiguredCustomInference,
    LocalMcp,
    LocalFilesystem,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("local-only runtime blocked official Warp backend request: operation={operation}, endpoint={endpoint_category}, subsystem={subsystem}")]
pub struct LocalOnlyBackendBlocked {
    pub operation: String,
    pub endpoint_category: String,
    pub subsystem: String,
}

impl LocalOnlyBackendBlocked {
    pub fn new(
        operation: impl Into<String>,
        endpoint_category: impl Into<String>,
        subsystem: impl Into<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            endpoint_category: endpoint_category.into(),
            subsystem: subsystem.into(),
        }
    }
}

impl fmt::Display for RuntimeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeMode::LocalOnly => f.write_str("local-only"),
            RuntimeMode::Connected => f.write_str("connected"),
        }
    }
}

const OVERRIDE_UNSET: u8 = 0;
const OVERRIDE_LOCAL_ONLY: u8 = 1;
const OVERRIDE_CONNECTED: u8 = 2;

static RUNTIME_MODE_OVERRIDE: AtomicU8 = AtomicU8::new(OVERRIDE_UNSET);

pub fn runtime_mode() -> RuntimeMode {
    match RUNTIME_MODE_OVERRIDE.load(Ordering::Relaxed) {
        OVERRIDE_LOCAL_ONLY => RuntimeMode::LocalOnly,
        OVERRIDE_CONNECTED => RuntimeMode::Connected,
        _ => RuntimeMode::LocalOnly,
    }
}

pub fn is_local_only() -> bool {
    runtime_mode() == RuntimeMode::LocalOnly
}

pub fn network_egress_policy() -> NetworkEgressPolicy {
    match runtime_mode() {
        RuntimeMode::LocalOnly => NetworkEgressPolicy::DenyOfficialWarpBackend,
        RuntimeMode::Connected => NetworkEgressPolicy::AllowOfficialWarpBackend,
    }
}

pub fn telemetry_egress_policy() -> TelemetryEgressPolicy {
    match runtime_mode() {
        RuntimeMode::LocalOnly => TelemetryEgressPolicy::Disabled,
        RuntimeMode::Connected => TelemetryEgressPolicy::Enabled,
    }
}

pub fn should_block_official_warp_backend() -> bool {
    network_egress_policy() == NetworkEgressPolicy::DenyOfficialWarpBackend
}

pub fn block_official_warp_backend(
    operation: impl Into<String>,
    endpoint_category: impl Into<String>,
    subsystem: impl Into<String>,
) -> Result<(), LocalOnlyBackendBlocked> {
    if should_block_official_warp_backend() {
        Err(LocalOnlyBackendBlocked::new(
            operation,
            endpoint_category,
            subsystem,
        ))
    } else {
        Ok(())
    }
}

pub fn block_official_warp_backend_url(
    operation: impl Into<String>,
    endpoint_category: impl Into<String>,
    subsystem: impl Into<String>,
    url: &str,
) -> Result<(), LocalOnlyBackendBlocked> {
    if should_block_official_warp_backend() && is_official_warp_backend_url(url) {
        Err(LocalOnlyBackendBlocked::new(
            operation,
            endpoint_category,
            subsystem,
        ))
    } else {
        Ok(())
    }
}

pub fn is_official_warp_backend_url(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(is_official_warp_backend_host))
        .unwrap_or(false)
}

pub fn is_official_warp_backend_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    matches!(
        host.as_str(),
        "app.warp.dev"
            | "rtc.app.warp.dev"
            | "sessions.app.warp.dev"
            | "staging.warp.dev"
            | "oz.warp.dev"
    )
}

#[cfg(any(test, feature = "test-util"))]
pub struct RuntimeModeOverride {
    previous: u8,
}

#[cfg(any(test, feature = "test-util"))]
impl RuntimeModeOverride {
    pub fn connected() -> Self {
        Self::set(RuntimeMode::Connected)
    }

    pub fn local_only() -> Self {
        Self::set(RuntimeMode::LocalOnly)
    }

    fn set(mode: RuntimeMode) -> Self {
        let value = match mode {
            RuntimeMode::LocalOnly => OVERRIDE_LOCAL_ONLY,
            RuntimeMode::Connected => OVERRIDE_CONNECTED,
        };
        let previous = RUNTIME_MODE_OVERRIDE.swap(value, Ordering::Relaxed);
        Self { previous }
    }
}

#[cfg(any(test, feature = "test-util"))]
impl Drop for RuntimeModeOverride {
    fn drop(&mut self) {
        RUNTIME_MODE_OVERRIDE.store(self.previous, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static RUNTIME_MODE_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn defaults_to_local_only() {
        let _lock = RUNTIME_MODE_TEST_LOCK.lock().unwrap();

        assert_eq!(runtime_mode(), RuntimeMode::LocalOnly);
        assert_eq!(
            network_egress_policy(),
            NetworkEgressPolicy::DenyOfficialWarpBackend
        );
        assert_eq!(telemetry_egress_policy(), TelemetryEgressPolicy::Disabled);
    }

    #[test]
    fn connected_override_allows_official_backend() {
        let _lock = RUNTIME_MODE_TEST_LOCK.lock().unwrap();
        let _override = RuntimeModeOverride::connected();

        assert_eq!(runtime_mode(), RuntimeMode::Connected);
        assert!(block_official_warp_backend("op", "graphql", "test").is_ok());
    }

    #[test]
    fn local_only_blocks_official_backend() {
        let _lock = RUNTIME_MODE_TEST_LOCK.lock().unwrap();
        let error = block_official_warp_backend("GetUser", "graphql", "warp_graphql").unwrap_err();

        assert_eq!(error.operation, "GetUser");
        assert_eq!(error.endpoint_category, "graphql");
        assert_eq!(error.subsystem, "warp_graphql");
    }

    #[test]
    fn local_only_allows_non_official_urls() {
        let _lock = RUNTIME_MODE_TEST_LOCK.lock().unwrap();
        let _override = RuntimeModeOverride::local_only();

        block_official_warp_backend_url(
            "operation",
            "public_api",
            "test",
            "http://127.0.0.1:8080/api/v1/test",
        )
        .expect("localhost test endpoints should not be blocked by official backend policy");
    }

    #[test]
    fn local_only_blocks_official_urls() {
        let _lock = RUNTIME_MODE_TEST_LOCK.lock().unwrap();
        let _override = RuntimeModeOverride::local_only();
        let error = block_official_warp_backend_url(
            "operation",
            "graphql",
            "test",
            "https://app.warp.dev/graphql/v2",
        )
        .unwrap_err();

        assert_eq!(error.endpoint_category, "graphql");
    }
}
