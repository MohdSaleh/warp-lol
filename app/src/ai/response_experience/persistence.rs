use diesel::prelude::*;

use super::event::RichAgentEvent;
use crate::persistence::model::NewLocalAgentRichEventRecord;
use crate::persistence::schema::local_agent_rich_event_records;

pub struct LocalAgentHistoryModel;

impl LocalAgentHistoryModel {
    pub fn insert_rich_events(
        conn: &mut SqliteConnection,
        events: &[RichAgentEvent],
    ) -> Result<(), diesel::result::Error> {
        if events.is_empty() {
            return Ok(());
        }

        let new_records: Vec<NewLocalAgentRichEventRecord> = events
            .iter()
            .map(|event| NewLocalAgentRichEventRecord {
                conversation_id: event.conversation_id.to_string(),
                run_id: event.run_id.to_string(),
                stream_id: event.stream_id.to_string(),
                sequence: event.sequence as i64,
                event_id: event.event_id.to_string(),
                schema_version: event.schema_version as i32,
                event_type: event.event_type.to_string(),
                payload_json: serde_json::to_string(&event.payload).unwrap_or_default(),
                occurred_at: event
                    .occurred_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
            })
            .collect();

        diesel::insert_into(local_agent_rich_event_records::table)
            .values(&new_records)
            .execute(conn)?;

        Ok(())
    }
}
