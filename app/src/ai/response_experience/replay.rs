use std::time::{Duration, SystemTime};

use diesel::prelude::*;

use super::durability::EventDurability;
use super::event::{AIAgentExchangeId, RichAgentEvent, RichEventPayload, RunId, TurnId};
use super::projection::RichResponseProjection;
use super::reducer::RichEventReducer;
use super::source::RichEventSource;
use super::visibility::EventVisibility;
use crate::persistence::model::LocalAgentRichEventRecord;
use crate::persistence::schema::local_agent_rich_event_records;
use crate::AIConversationId;

#[allow(dead_code)]
pub struct ReplayService;

#[allow(dead_code)]
impl ReplayService {
    pub fn load_and_reduce(
        conn: &mut diesel::sqlite::SqliteConnection,
        conversation_id: &AIConversationId,
    ) -> Result<RichResponseProjection, diesel::result::Error> {
        let records: Vec<LocalAgentRichEventRecord> = local_agent_rich_event_records::table
            .filter(local_agent_rich_event_records::conversation_id.eq(conversation_id.to_string()))
            .order(local_agent_rich_event_records::sequence.asc())
            .load(conn)?;

        let mut reducer = RichEventReducer::new();
        for record in records {
            if let Ok(payload) = serde_json::from_str::<RichEventPayload>(&record.payload_json) {
                let event = RichAgentEvent {
                    schema_version: record.schema_version as u32,
                    event_id: record.event_id,
                    event_type: record.event_type,
                    stream_id: record.stream_id,
                    sequence: record.sequence as u64,
                    entity_revision: 0,
                    run_id: record.run_id,
                    turn_id: TurnId::new(), // Dummy
                    conversation_id: *conversation_id,
                    task_id: None,
                    exchange_id: None,
                    parent_event_id: None,
                    correlation_id: None,
                    occurred_at: SystemTime::UNIX_EPOCH
                        + Duration::from_millis(record.occurred_at as u64),
                    source: RichEventSource::LocalRuntime, // Dummy
                    visibility: EventVisibility::UserVisible, // Dummy
                    durability: EventDurability::Transient, // Dummy
                    payload,
                };
                reducer.apply(event);
            }
        }

        Ok(reducer.state)
    }
}
