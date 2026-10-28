use serde::{Deserialize, Serialize};

use super::errors::NormalizedErrorType;
use crate::ai::response_experience::source::{ProviderId, ProviderModelRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunOutcomeClass {
    Success,
    PartialSuccess,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactedLatencySummary {
    pub time_to_first_feedback_ms: Option<u64>,
    pub time_to_first_text_ms: Option<u64>,
    pub wall_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedUsageSummary {
    pub provider_reported: bool,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDiagnosticReport {
    pub report_version: u32,

    pub run_id_hash: String,
    pub trace_id: Option<String>,

    pub provider: Option<ProviderId>,
    pub model: Option<ProviderModelRef>,

    pub outcome: RunOutcomeClass,
    pub error_type: Option<NormalizedErrorType>,

    pub attempts: u32,
    pub retries: u32,
    pub fallbacks: u32,

    pub latency: RedactedLatencySummary,
    pub usage: RedactedUsageSummary,

    pub provider_request_ids: Vec<String>,

    pub app_version: String,
    pub platform: String,
}

impl AgentDiagnosticReport {
    pub fn to_human_readable_text(&self) -> String {
        let mut out = String::new();
        out.push_str("Warp Agent Diagnostic Report\n");
        out.push_str(&format!("Report version: {}\n", self.report_version));
        out.push_str(&format!("App version: {}\n", self.app_version));
        out.push_str(&format!("Platform: {}\n", self.platform));
        out.push_str(&format!("Run reference: {}\n", self.run_id_hash));

        if let Some(trace_id) = &self.trace_id {
            out.push_str(&format!("Trace ID: {}\n", trace_id));
        }

        out.push_str(&format!("Outcome: {:?}\n", self.outcome));

        if let Some(provider) = &self.provider {
            out.push_str(&format!("Provider: {}\n", provider.as_str()));
        } else {
            out.push_str("Provider: unknown\n");
        }

        if let Some(model) = &self.model {
            out.push_str(&format!("Model: {}\n", model));
        } else {
            out.push_str("Model: unknown\n");
        }

        out.push_str(&format!("Attempts: {}\n", self.attempts));
        out.push_str(&format!("Retries: {}\n", self.retries));
        out.push_str(&format!("Fallbacks: {}\n", self.fallbacks));

        if let Some(e) = &self.error_type {
            out.push_str(&format!("Error class: {:?}\n", e));
        }

        if let Some(t) = self.latency.time_to_first_feedback_ms {
            out.push_str(&format!("Time to first feedback: {} ms\n", t));
        }
        if let Some(t) = self.latency.time_to_first_text_ms {
            out.push_str(&format!("Time to first text: {} ms\n", t));
        }
        out.push_str(&format!("Wall time: {} ms\n", self.latency.wall_time_ms));

        out.push_str(&format!(
            "Token usage: {}\n",
            if self.usage.provider_reported {
                "provider reported"
            } else {
                "locally estimated"
            }
        ));
        if let Some(i) = self.usage.input_tokens {
            out.push_str(&format!("Input tokens: {}\n", i));
        }
        if let Some(o) = self.usage.output_tokens {
            out.push_str(&format!("Output tokens: {}\n", o));
        }

        if !self.provider_request_ids.is_empty() {
            out.push_str(&format!(
                "Provider request IDs: {}\n",
                self.provider_request_ids.join(", ")
            ));
        }

        out
    }
}
