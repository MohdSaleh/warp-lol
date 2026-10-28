CREATE TABLE local_agent_rich_event_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    conversation_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    stream_id TEXT NOT NULL,

    sequence BIGINT NOT NULL,
    event_id TEXT NOT NULL,

    schema_version INTEGER NOT NULL,
    event_type TEXT NOT NULL,

    payload_json TEXT NOT NULL,
    occurred_at BIGINT NOT NULL,

    UNIQUE(stream_id, sequence)
);

CREATE TABLE local_agent_rich_projection_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    conversation_id TEXT NOT NULL,
    run_id TEXT NOT NULL,

    schema_version INTEGER NOT NULL,
    last_sequences_json TEXT NOT NULL,

    projection_json TEXT NOT NULL,
    projection_digest TEXT NOT NULL,

    updated_at BIGINT NOT NULL,

    UNIQUE(run_id)
);
