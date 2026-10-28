use std::str::FromStr;

use diesel::prelude::*;

use crate::persistence::model::{AgentCostEntry, AgentUsageEntry};
use crate::persistence::schema::{agent_cost_entries, agent_usage_entries};

#[derive(Debug, Default)]
pub struct ConversationUsageRows {
    pub credits_spent: Option<f32>,
    pub models: Vec<crate::persistence::model::ModelTokenUsage>,
    // TODO: tool calls and others if needed, but for now just cost/tokens
}

pub fn get_conversation_usage(
    conn: &mut diesel::sqlite::SqliteConnection,
    conversation_id: &str,
) -> QueryResult<ConversationUsageRows> {
    let usages: Vec<AgentUsageEntry> = agent_usage_entries::table
        .filter(agent_usage_entries::conversation_id.eq(conversation_id))
        .load(conn)?;

    let mut total_cost = 0.0;
    let mut has_unpriced = false;
    let mut models_map = std::collections::HashMap::new();

    for usage in usages {
        let cost: Option<AgentCostEntry> = agent_cost_entries::table
            .filter(agent_cost_entries::usage_entry_id.eq(&usage.usage_entry_id))
            .first(conn)
            .optional()?;

        if let Some(c) = cost {
            if c.classification == "Unpriced" {
                has_unpriced = true;
            } else if let Some(amt) = c.amount {
                if let Ok(f) = f32::from_str(&amt) {
                    total_cost += f;
                }
            }
        }
        let token_usage: crate::ai::observability::usage::entry::CanonicalTokenUsage =
            serde_json::from_str(&usage.token_usage_json).unwrap_or_default();

        let entry = models_map.entry(usage.model_id.clone()).or_insert_with(|| {
            crate::persistence::model::ModelTokenUsage {
                model_id: usage.model_id.clone(),
                ..Default::default()
            }
        });

        entry.warp_tokens +=
            (token_usage.input_tokens.unwrap_or(0) + token_usage.output_tokens.unwrap_or(0)) as u32;
    }

    Ok(ConversationUsageRows {
        credits_spent: if has_unpriced { None } else { Some(total_cost) },
        models: models_map.into_values().collect(),
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use diesel::prelude::*;
    use uuid::Uuid;

    use super::*;
    use crate::ai::observability::usage::entry::CanonicalTokenUsage;
    use crate::persistence::model::{AgentCostEntry, AgentUsageEntry};
    use crate::persistence::schema::{agent_cost_entries, agent_usage_entries};

    #[test]
    fn test_conversation_usage() {
        use diesel_migrations::MigrationHarness;
        let mut conn = diesel::sqlite::SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(::persistence::MIGRATIONS)
            .unwrap();

        let conv_id = Uuid::new_v4().to_string();

        let token_usage = CanonicalTokenUsage {
            input_tokens: Some(10),
            output_tokens: Some(20),
            ..Default::default()
        };
        let token_json = serde_json::to_string(&token_usage).unwrap();

        let usage_id_1 = Uuid::new_v4().to_string();
        diesel::insert_into(agent_usage_entries::table)
            .values(&AgentUsageEntry {
                usage_entry_id: usage_id_1.clone(),
                conversation_id: conv_id.clone(),
                run_id: Uuid::new_v4().to_string(),
                turn_id: Uuid::new_v4().to_string(),
                logical_request_id: Uuid::new_v4().to_string(),
                attempt_id: Uuid::new_v4().to_string(),
                provider_id: "warp".to_string(),
                model_id: "gpt-4".to_string(),
                route_type: "Primary".to_string(),
                source: "User".to_string(),
                status: "Success".to_string(),
                token_usage_json: token_json.clone(),
                provider_breakdown_json: "{}".to_string(),
                provider_request_id: None,
                supersedes_entry_id: None,
                observed_at: chrono::Utc::now().naive_utc(),
            })
            .execute(&mut conn)
            .unwrap();

        diesel::insert_into(agent_cost_entries::table)
            .values(&AgentCostEntry {
                cost_entry_id: Uuid::new_v4().to_string(),
                usage_entry_id: usage_id_1.clone(),
                amount: Some("0.10".to_string()),
                currency: Some("USD".to_string()),
                classification: "Priced".to_string(),
                pricing_catalog_version: Some("v1".to_string()),
                calculation_json: Some("{}".to_string()),
                created_at: chrono::Utc::now().naive_utc(),
            })
            .execute(&mut conn)
            .unwrap();

        let usage = get_conversation_usage(&mut conn, &conv_id).unwrap();
        assert_eq!(usage.credits_spent, Some(0.10));
        assert_eq!(usage.models.len(), 1);
        assert_eq!(usage.models[0].model_id, "gpt-4");
        assert_eq!(usage.models[0].warp_tokens, 30);
    }
}
