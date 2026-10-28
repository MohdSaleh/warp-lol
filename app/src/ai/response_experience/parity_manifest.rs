use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityManifest {
    pub feature: Vec<FeatureManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureManifest {
    pub id: String,
    pub cloud_source: String,
    pub local_source: String,
    pub canonical_event: String,
    pub renderer: String,
    pub persistence: String,
    pub status: String,
    pub reason: Option<String>,
}

impl ParityManifest {
    pub fn load() -> Self {
        let toml_str = include_str!("parity_manifest.toml");
        toml::from_str(toml_str).expect("Failed to parse parity_manifest.toml")
    }
}
