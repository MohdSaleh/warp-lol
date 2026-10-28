use serde::{Deserialize, Serialize};

use super::catalog::CurrencyCode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostRollup {
    pub total_amount: Option<f64>,
    pub currency: Option<CurrencyCode>,
    pub fully_calculated: bool,
}
