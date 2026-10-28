use super::provider::{
    DiscoveredRule, NormalizedRule, RuleEnforcementMode, RuleId, RuleNormalizationError,
    RuleProvider, RuleProviderId, RuleProviderPolicy, RuleScope,
};
use super::validation::RuleValidationReport;

pub struct WarpRuleProvider;

impl RuleProvider for WarpRuleProvider {
    fn provider_id(&self) -> RuleProviderId {
        RuleProviderId("warp".to_string())
    }

    fn normalize(&self, rule: DiscoveredRule) -> Result<NormalizedRule, RuleNormalizationError> {
        let path_str = rule.path.display_path();
        Ok(NormalizedRule {
            id: RuleId(path_str.clone()),
            provider: self.provider_id(),
            scope: RuleScope {
                path: rule.path.clone(),
            },
            enforcement: RuleEnforcementMode::Advisory,
            content: rule.content,
            validation: RuleValidationReport::default(),
        })
    }

    fn precedence_policy(&self) -> RuleProviderPolicy {
        RuleProviderPolicy {}
    }
}
