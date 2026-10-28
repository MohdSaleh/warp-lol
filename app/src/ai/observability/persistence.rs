use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRetentionPolicy {
    pub observation_event_retention: Duration,
    pub detailed_usage_retention: Duration,

    pub cleanup_batch_size: usize,
    pub maximum_cleanup_duration: Duration,

    pub database_soft_limit_bytes: u64,
    pub database_hard_limit_bytes: u64,
}

impl Default for UsageRetentionPolicy {
    fn default() -> Self {
        Self {
            observation_event_retention: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            detailed_usage_retention: Duration::from_secs(180 * 24 * 60 * 60),   // 180 days
            cleanup_batch_size: 1000,
            maximum_cleanup_duration: Duration::from_millis(250),
            database_soft_limit_bytes: 50 * 1024 * 1024, // 50MB
            database_hard_limit_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}
