use std::collections::BTreeMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::ai::response_experience::source::ProviderId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum CurrencyCode {
    USD,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMatchRule {
    pub exact_match: Option<String>,
    pub pattern_match: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPricingEntry {
    pub provider: ProviderId,
    pub canonical_model_id: String,
    pub model_aliases: Vec<String>,

    pub currency: CurrencyCode,

    pub input_per_million: Option<f64>,
    pub cached_input_per_million: Option<f64>,
    pub cache_write_per_million: Option<f64>,
    pub output_per_million: Option<f64>,
    pub reasoning_per_million: Option<f64>,

    pub server_tool_rates: BTreeMap<String, f64>,

    pub effective_at: SystemTime,
    pub retired_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingCatalog {
    pub schema_version: u32,
    pub catalog_version: u64,

    pub generated_at: SystemTime,
    pub effective_at: SystemTime,
    pub expires_at: SystemTime,

    pub entries: Vec<ModelPricingEntry>,

    pub key_id: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedPricingCatalog(pub PricingCatalog);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivePricingCatalog(pub PricingCatalog);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CatalogRefreshState {
    Idle,
    Refreshing,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPricingCatalogModel {
    pub embedded: PricingCatalog,
    pub cached: Option<VerifiedPricingCatalog>,
    pub active: ActivePricingCatalog,
    pub refresh_state: CatalogRefreshState,
}
