//! Reconstructed loop-control state for the current root user request.
//!
//! The local brain is invoked once per provider turn and does not retain an
//! in-memory run object across tool-result continuations. `Ledger::for_request`
//! therefore rebuilds counters from the ordered messages in `RequestParams`:
//! the latest persisted user query is the root boundary, and only tool calls
//! after that boundary count. A fresh user query not yet present in history
//! starts a pending root with zero prior turns. No database state is used.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, SystemTime};

use diesel::prelude::*;
use persistence::model::LocalAgentRuntimeRecord;
use persistence::schema::local_agent_runtime_records;
use serde::{Deserialize, Serialize};
use warp_multi_agent_api as api;

use super::fingerprint::{fingerprint_tool_call, hash_history};

const RECENT_TOOL_CALL_LIMIT: usize = 32;
const PENDING_ROOT_USER_MESSAGE_ID: &str = "pending-user-query";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalAgentRunStatus {
    Active,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub call_id: String,
    pub fingerprint_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRecord {
    pub call_id: String,
    pub fingerprint_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressRecord {
    pub progressed: bool,
    pub batch_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCheckpointRef {
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalAgentTerminationReason {
    ContextWindowExceeded,
    CompactionFailed,
    RepeatedToolLoop,
    NoMeasurableProgress,
    CompletedSuccessfully,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub status: LocalAgentRunStatus,
    pub root_user_message_id: String,
    pub started_at_wall_clock: SystemTime,
    pub accumulated_active_duration: Duration,
    pub model_turns: u32,
    pub tool_calls: u32,
    pub tool_batches: u32,
    pub provider_continuations: u32,
    pub completion_repair_turns: u32,
    pub compaction_count: u32,
    pub cumulative_input_tokens: u64,
    pub cumulative_output_tokens: u64,
    pub consecutive_deterministic_failures: u32,
    pub consecutive_no_progress_batches: u32,
    pub recent_tool_calls: VecDeque<ToolCallRecord>,
    pub recent_observations: VecDeque<ObservationRecord>,
    pub recent_progress: VecDeque<ProgressRecord>,
    pub current_checkpoint: Option<ContextCheckpointRef>,
    pub request_epoch: u64,
    pub history_digest: String,
    pub terminal_reason: Option<LocalAgentTerminationReason>,
    pub applied_runtime_event_ids: VecDeque<String>,
}

pub type LocalAgentRunLedger = Ledger;

impl Default for Ledger {
    fn default() -> Self {
        Self {
            status: LocalAgentRunStatus::Active,
            root_user_message_id: String::new(),
            started_at_wall_clock: SystemTime::now(),
            accumulated_active_duration: Duration::ZERO,
            model_turns: 0,
            tool_calls: 0,
            tool_batches: 0,
            provider_continuations: 0,
            completion_repair_turns: 0,
            compaction_count: 0,
            cumulative_input_tokens: 0,
            cumulative_output_tokens: 0,
            consecutive_deterministic_failures: 0,
            consecutive_no_progress_batches: 0,
            recent_tool_calls: VecDeque::new(),
            recent_observations: VecDeque::new(),
            recent_progress: VecDeque::new(),
            current_checkpoint: None,
            request_epoch: 0,
            history_digest: String::new(),
            terminal_reason: None,
            applied_runtime_event_ids: VecDeque::new(),
        }
    }
}

impl Ledger {
    pub fn for_request(tasks: &[api::Task], current_user_query: Option<&str>) -> Self {
        let messages = tasks
            .iter()
            .flat_map(|task| task.messages.iter())
            .collect::<Vec<_>>();
        let latest_user = messages
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, message)| match message.message.as_ref() {
                Some(api::message::Message::UserQuery(query)) => Some((index, *message, query)),
                _ => None,
            });

        let persisted_root = match (current_user_query, latest_user) {
            (Some(query), Some((index, message, user_query)))
                if user_query.query == query && !has_activity_after(&messages[index + 1..]) =>
            {
                Some((index, message))
            }
            (Some(_), _) => None,
            (None, Some((index, message, _))) => Some((index, message)),
            (None, None) => None,
        };

        let mut ledger = Self {
            history_digest: hash_history(tasks),
            ..Self::default()
        };
        let Some((root_index, root_message)) = persisted_root else {
            if current_user_query.is_some() {
                ledger.root_user_message_id = PENDING_ROOT_USER_MESSAGE_ID.to_string();
            }
            return ledger;
        };

        ledger.root_user_message_id = root_message.id.clone();
        let mut provider_requests = HashSet::new();
        for message in messages.into_iter().skip(root_index + 1) {
            let Some(api::message::Message::ToolCall(call)) = message.message.as_ref() else {
                continue;
            };

            ledger.tool_calls = ledger.tool_calls.saturating_add(1);
            if message.request_id.is_empty() || provider_requests.insert(message.request_id.clone())
            {
                ledger.model_turns = ledger.model_turns.saturating_add(1);
            }
            ledger.recent_tool_calls.push_back(ToolCallRecord {
                call_id: call.tool_call_id.clone(),
                fingerprint_hash: fingerprint_tool_call(call),
            });
            if ledger.recent_tool_calls.len() > RECENT_TOOL_CALL_LIMIT {
                ledger.recent_tool_calls.pop_front();
            }
        }
        ledger.tool_batches = ledger.model_turns;
        ledger
    }

    /// Record tool calls the harness executed itself, so repeated-loop and
    /// no-progress guards see them exactly as they see client-executed calls.
    pub fn record_tool_calls<I>(&mut self, calls: I)
    where
        I: IntoIterator<Item = (String, String)>,
    {
        for (call_id, fingerprint_hash) in calls {
            self.recent_tool_calls.push_back(ToolCallRecord {
                call_id,
                fingerprint_hash,
            });
            while self.recent_tool_calls.len() > RECENT_TOOL_CALL_LIMIT {
                self.recent_tool_calls.pop_front();
            }
        }
    }

    pub fn to_runtime_record(
        &self,
        conversation_id: &str,
        run_id: &str,
    ) -> LocalAgentRuntimeRecord {
        let now = chrono::Utc::now().naive_utc();
        let status_str = match self.status {
            LocalAgentRunStatus::Active => "Active",
            LocalAgentRunStatus::Terminated => "Terminated",
        };
        LocalAgentRuntimeRecord {
            conversation_id: conversation_id.to_string(),
            run_id: run_id.to_string(),
            schema_version: 1,
            request_epoch: self.request_epoch as i32,
            status: status_str.to_string(),
            history_digest: self.history_digest.clone(),
            ledger_json: serde_json::to_string(self).unwrap_or_default(),
            created_at: now,
            last_modified_at: now,
        }
    }
}

pub fn load_runtime_record(
    conn: &mut SqliteConnection,
    conversation_id: &str,
    run_id: &str,
) -> anyhow::Result<Option<LocalAgentRuntimeRecord>> {
    let record = local_agent_runtime_records::table
        .filter(local_agent_runtime_records::conversation_id.eq(conversation_id))
        .filter(local_agent_runtime_records::run_id.eq(run_id))
        .first::<LocalAgentRuntimeRecord>(conn)
        .optional()?;
    Ok(record)
}

pub fn save_runtime_record(
    conn: &mut SqliteConnection,
    record: &LocalAgentRuntimeRecord,
) -> anyhow::Result<()> {
    diesel::insert_into(local_agent_runtime_records::table)
        .values(record)
        .on_conflict((
            local_agent_runtime_records::conversation_id,
            local_agent_runtime_records::run_id,
        ))
        .do_update()
        .set((
            local_agent_runtime_records::status.eq(&record.status),
            local_agent_runtime_records::history_digest.eq(&record.history_digest),
            local_agent_runtime_records::ledger_json.eq(&record.ledger_json),
            local_agent_runtime_records::request_epoch.eq(&record.request_epoch),
            local_agent_runtime_records::last_modified_at.eq(&record.last_modified_at),
        ))
        .execute(conn)?;
    Ok(())
}

fn has_activity_after(messages: &[&api::Message]) -> bool {
    messages.iter().any(|message| {
        matches!(
            message.message.as_ref(),
            Some(api::message::Message::AgentOutput(_))
                | Some(api::message::Message::ToolCall(_))
                | Some(api::message::Message::ToolCallResult(_))
        )
    })
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
