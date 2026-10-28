use warp_util::local_or_remote_path::LocalOrRemotePath;

use super::validation::RuleValidationReport;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleProviderId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleId(pub String);

#[derive(Debug, Clone)]
pub struct RuleScope {
    pub path: LocalOrRemotePath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleEnforcementMode {
    Advisory,
    RequiredForPreflight,
    HostEnforced,
}

#[derive(Debug, Clone)]
pub struct NormalizedRule {
    pub id: RuleId,
    pub provider: RuleProviderId,
    pub scope: RuleScope,
    pub enforcement: RuleEnforcementMode,
    pub content: String,
    pub validation: RuleValidationReport,
}

#[derive(Debug, Clone)]
pub struct DiscoveredRule {
    pub path: LocalOrRemotePath,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct RuleProviderPolicy {
    // Defines how precedence works for this provider
}

#[derive(Debug, Clone)]
pub struct RuleDiscoveryError;
#[derive(Debug, Clone)]
pub struct RuleNormalizationError;

pub trait RuleProvider {
    fn provider_id(&self) -> RuleProviderId;

    fn normalize(&self, rule: DiscoveredRule) -> Result<NormalizedRule, RuleNormalizationError>;

    fn precedence_policy(&self) -> RuleProviderPolicy;
}
