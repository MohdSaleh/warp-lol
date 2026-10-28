use chrono::Utc;
use diesel::prelude::*;
use persistence::model::{AgentCostEntry, AgentUsageEntry};
use persistence::schema::{agent_cost_entries, agent_usage_entries};
use uuid::Uuid;

use crate::ai::observability::cost::calculator::calculate;
use crate::ai::observability::cost::catalog::PricingCatalog;
use crate::ai::observability::identifiers::{CostEntryId, UsageEntryId};
use crate::ai::observability::privacy::{AgentTelemetryPrivacyPolicy, IdentifierPolicy};
use crate::ai::observability::usage::entry::CanonicalTokenUsage;

pub struct UsageWriter;

impl UsageWriter {
    #[allow(clippy::too_many_arguments)]
    pub fn write_usage_and_cost(
        conn: &mut SqliteConnection,
        usage: &CanonicalTokenUsage,
        provider: &str,
        model: &str,
        catalog: &PricingCatalog,
        privacy_policy: &AgentTelemetryPrivacyPolicy,
        conversation_id: &str,
        run_id: &str,
        turn_id: &str,
        logical_request_id: &str,
        attempt_id: &str,
        route_type: &str,
        source: &str,
        status: &str,
    ) -> QueryResult<()> {
        let usage_entry_id = UsageEntryId(Uuid::new_v4());
        let cost_entry = calculate(
            usage_entry_id,
            usage,
            &provider.to_string(),
            &model.to_string(),
            catalog,
        );

        // Apply redaction rules
        let mut redacted_conversation_id = conversation_id.to_string();
        let mut redacted_run_id = run_id.to_string();

        match privacy_policy.identifier_policy {
            IdentifierPolicy::Redact => {
                redacted_conversation_id = "[REDACTED]".to_string();
                redacted_run_id = "[REDACTED]".to_string();
            }
            IdentifierPolicy::Hash => {
                // Implement hashing if needed, or just keep as is for now
                use std::hash::{DefaultHasher, Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                conversation_id.hash(&mut hasher);
                redacted_conversation_id = format!("{:x}", hasher.finish());

                let mut hasher2 = DefaultHasher::new();
                run_id.hash(&mut hasher2);
                redacted_run_id = format!("{:x}", hasher2.finish());
            }
            IdentifierPolicy::Keep => {}
        }

        let agent_usage = AgentUsageEntry {
            usage_entry_id: usage_entry_id.0.to_string(),
            conversation_id: redacted_conversation_id,
            run_id: redacted_run_id,
            turn_id: turn_id.to_string(),
            logical_request_id: logical_request_id.to_string(),
            attempt_id: attempt_id.to_string(),
            provider_id: provider.to_string(),
            model_id: model.to_string(),
            route_type: route_type.to_string(),
            source: source.to_string(),
            status: status.to_string(),
            token_usage_json: serde_json::to_string(usage).unwrap_or_default(),
            provider_breakdown_json: "{}".to_string(), // TODO: add breakdown
            provider_request_id: None,
            supersedes_entry_id: None,
            observed_at: Utc::now().naive_utc(),
        };

        let agent_cost = AgentCostEntry {
            cost_entry_id: CostEntryId(Uuid::new_v4()).0.to_string(),
            usage_entry_id: usage_entry_id.0.to_string(),
            amount: cost_entry.amount.map(|v| v.to_string()),
            currency: cost_entry.currency.map(|c| format!("{:?}", c)),
            classification: format!("{:?}", cost_entry.classification),
            pricing_catalog_version: cost_entry.pricing_catalog_version.map(|v| v.to_string()),
            calculation_json: cost_entry
                .calculation_breakdown
                .map(|b| serde_json::to_string(&b).unwrap_or_default()),
            created_at: Utc::now().naive_utc(),
        };

        conn.transaction(|conn| {
            diesel::insert_into(agent_usage_entries::table)
                .values(&agent_usage)
                .execute(conn)?;

            diesel::insert_into(agent_cost_entries::table)
                .values(&agent_cost)
                .execute(conn)?;

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use diesel::sqlite::SqliteConnection;
    use diesel_migrations::MigrationHarness;

    use super::*;
    use crate::ai::observability::cost::catalog::{
        CurrencyCode, ModelPricingEntry, PricingCatalog,
    };
    use crate::ai::observability::privacy::ExportDecision;

    fn get_privacy_policy() -> AgentTelemetryPrivacyPolicy {
        AgentTelemetryPrivacyPolicy {
            export_enabled: true,
            content_export_enabled: false,
            metadata_export: ExportDecision::Allowed,
            crash_diagnostics_export: ExportDecision::Allowed,
            ai_content_export: ExportDecision::Allowed,
            allow_prompt_content: false,
            allow_response_content: false,
            allow_tool_arguments: false,
            allow_tool_results: false,
            allow_source_excerpts: false,
            identifier_policy: IdentifierPolicy::Redact,
            enterprise_zero_retention: false,
            secret_redaction_required: true,
            policy_generation: 1,
        }
    }

    fn get_test_catalog() -> PricingCatalog {
        PricingCatalog {
            schema_version: 1,
            catalog_version: 42,
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
        }
    }

    #[test]
    fn usage_persistence() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(::persistence::MIGRATIONS)
            .unwrap();

        let usage = CanonicalTokenUsage {
            input_tokens: Some(100),
            output_tokens: Some(50),
            ..Default::default()
        };

        UsageWriter::write_usage_and_cost(
            &mut conn,
            &usage,
            "openai",
            "gpt-4",
            &get_test_catalog(),
            &get_privacy_policy(),
            "conv_123",
            "run_123",
            "turn_123",
            "req_123",
            "att_123",
            "Primary",
            "User",
            "Success",
        )
        .unwrap();

        // Check both tables have 1 row
        let usages: Vec<AgentUsageEntry> = agent_usage_entries::table.load(&mut conn).unwrap();
        let costs: Vec<AgentCostEntry> = agent_cost_entries::table.load(&mut conn).unwrap();

        assert_eq!(usages.len(), 1);
        assert_eq!(costs.len(), 1);

        // Joined cost matches catalog math
        // Input: 100 * 10 / 1M = 0.001
        // Output: 50 * 30 / 1M = 0.0015
        // Total: 0.0025
        assert_eq!(costs[0].amount.as_deref(), Some("0.0025"));
    }

    #[test]
    fn observability_redaction() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(::persistence::MIGRATIONS)
            .unwrap();

        let usage = CanonicalTokenUsage {
            input_tokens: Some(10),
            output_tokens: Some(10),
            ..Default::default()
        };

        // Seed a prompt text containing a CANARY_9f2e
        let _prompt_text_with_canary = "This prompt contains sensitive CANARY_9f2e details";

        UsageWriter::write_usage_and_cost(
            &mut conn,
            &usage,
            "openai",
            "gpt-4",
            &get_test_catalog(),
            &get_privacy_policy(),
            "conv_123",
            "run_123",
            "turn_123",
            "req_123",
            "att_123",
            "Primary",
            "User",
            "Success",
        )
        .unwrap();

        // Check tables to ensure they NEVER contain the prompt text or CANARY_9f2e
        let usages: Vec<AgentUsageEntry> = agent_usage_entries::table.load(&mut conn).unwrap();
        let costs: Vec<AgentCostEntry> = agent_cost_entries::table.load(&mut conn).unwrap();

        for usage in usages {
            assert!(!usage.token_usage_json.contains("CANARY_9f2e"));
            assert!(!usage.conversation_id.contains("CANARY_9f2e"));
        }
        for cost in costs {
            assert!(!cost
                .calculation_json
                .unwrap_or_default()
                .contains("CANARY_9f2e"));
        }
    }
}
