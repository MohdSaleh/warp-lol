use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::catalog::CurrencyCode;
use crate::ai::observability::identifiers::{CostEntryId, UsageEntryId};
use crate::ai::response_experience::source::{ProviderId, ProviderModelRef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CostClassification {
    ProviderReported,
    Calculated,
    Estimated,
    EstimatedFromStaleCatalog,
    SubscriptionIncluded,
    WarpCredits,
    PlatformCredits,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostCalculationBreakdown {
    pub input_cost: Option<f64>,
    pub cached_input_cost: Option<f64>,
    pub cache_write_cost: Option<f64>,
    pub output_cost: Option<f64>,
    pub reasoning_cost: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostEntry {
    pub schema_version: u32,
    pub cost_entry_id: CostEntryId,
    pub usage_entry_id: UsageEntryId,

    pub provider: ProviderId,
    pub model: ProviderModelRef,

    pub amount: Option<f64>,
    pub currency: Option<CurrencyCode>,

    pub classification: CostClassification,
    pub pricing_catalog_version: Option<String>,
    pub pricing_effective_at: Option<SystemTime>,

    pub calculation_breakdown: Option<CostCalculationBreakdown>,
}
