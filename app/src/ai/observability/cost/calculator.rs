use uuid::Uuid;

use crate::ai::observability::cost::catalog::PricingCatalog;
use crate::ai::observability::cost::entry::{
    CostCalculationBreakdown, CostClassification, CostEntry,
};
use crate::ai::observability::identifiers::{CostEntryId, UsageEntryId};
use crate::ai::observability::usage::entry::CanonicalTokenUsage;
use crate::ai::response_experience::source::{ProviderId, ProviderModelRef};

pub fn calculate(
    usage_entry_id: UsageEntryId,
    usage: &CanonicalTokenUsage,
    provider: &ProviderId,
    model: &ProviderModelRef,
    catalog: &PricingCatalog,
) -> CostEntry {
    let mut amount = None;
    let mut classification = CostClassification::Unknown;
    let mut calculation_breakdown = None;
    let mut currency = None;
    let mut pricing_catalog_version = None;
    let mut pricing_effective_at = None;

    if let Some(entry) = catalog.entries.iter().find(|e| {
        e.provider == *provider
            && (e.canonical_model_id == *model || e.model_aliases.contains(model))
    }) {
        let in_cost = usage.input_tokens.unwrap_or(0) as f64
            * entry.input_per_million.unwrap_or(0.0)
            / 1_000_000.0;
        let out_cost = usage.output_tokens.unwrap_or(0) as f64
            * entry.output_per_million.unwrap_or(0.0)
            / 1_000_000.0;

        amount = Some(in_cost + out_cost);
        classification = CostClassification::Calculated;
        currency = Some(entry.currency.clone());
        pricing_catalog_version = Some(catalog.catalog_version.to_string());
        pricing_effective_at = Some(catalog.effective_at);

        calculation_breakdown = Some(CostCalculationBreakdown {
            input_cost: Some(in_cost),
            cached_input_cost: None,
            cache_write_cost: None,
            output_cost: Some(out_cost),
            reasoning_cost: None,
        });
    }

    CostEntry {
        schema_version: 1,
        cost_entry_id: CostEntryId(Uuid::new_v4()),
        usage_entry_id,
        provider: provider.clone(),
        model: model.clone(),
        amount,
        currency,
        classification,
        pricing_catalog_version,
        pricing_effective_at,
        calculation_breakdown,
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use uuid::Uuid;

    use super::*;
    use crate::ai::observability::cost::catalog::{
        CurrencyCode, ModelPricingEntry, PricingCatalog,
    };
    use crate::ai::observability::identifiers::UsageEntryId;
    use crate::ai::observability::usage::entry::CanonicalTokenUsage;

    #[test]
    fn cost_calculator() {
        let catalog = PricingCatalog {
            schema_version: 1,
            catalog_version: 1,
            generated_at: SystemTime::now(),
            effective_at: SystemTime::now(),
            expires_at: SystemTime::now(),
            entries: vec![ModelPricingEntry {
                provider: "openai".to_string(),
                canonical_model_id: "gpt-4".to_string(),
                model_aliases: vec![],
                currency: CurrencyCode::USD,
                input_per_million: Some(10.0),
                cached_input_per_million: None,
                cache_write_per_million: None,
                output_per_million: Some(30.0),
                reasoning_per_million: None,
                server_tool_rates: Default::default(),
                effective_at: SystemTime::now(),
                retired_at: None,
            }],
            key_id: "test".to_string(),
            signature: "sig".to_string(),
        };

        let usage = CanonicalTokenUsage {
            input_tokens: Some(1_000_000), // $10.0
            output_tokens: Some(500_000),  // $15.0
            cached_input_tokens: None,
            cache_write_tokens: None,
            reasoning_tokens: None,
            audio_input_tokens: None,
            audio_output_tokens: None,
            image_input_tokens: None,
            total_tokens: Some(1_500_000),
        };

        // Exact known model match
        let entry = calculate(
            UsageEntryId(Uuid::new_v4()),
            &usage,
            &"openai".to_string(),
            &"gpt-4".to_string(),
            &catalog,
        );
        assert_eq!(entry.classification, CostClassification::Calculated);
        assert_eq!(entry.amount, Some(25.0));

        // Unknown model
        let unknown = calculate(
            UsageEntryId(Uuid::new_v4()),
            &usage,
            &"openai".to_string(),
            &"unknown-model".to_string(),
            &catalog,
        );
        assert_eq!(unknown.classification, CostClassification::Unknown);
        assert_eq!(unknown.amount, None);
    }
}
