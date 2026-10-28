use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenAISemanticConventionVersion {
    pub schema_url: String,
    pub internal_version: u32,
}
