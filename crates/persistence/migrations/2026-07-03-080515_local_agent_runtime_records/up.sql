CREATE TABLE local_agent_runtime_records (
    conversation_id TEXT NOT NULL,
    run_id TEXT NOT NULL,

    schema_version INTEGER NOT NULL,
    request_epoch INTEGER NOT NULL,

    status TEXT NOT NULL,
    history_digest TEXT NOT NULL,

    ledger_json TEXT NOT NULL,

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_modified_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY (conversation_id, run_id),

    FOREIGN KEY (conversation_id)
        REFERENCES agent_conversations (conversation_id)
        ON DELETE CASCADE
);

CREATE INDEX ix_local_agent_runtime_conversation
    ON local_agent_runtime_records (conversation_id);

CREATE INDEX ix_local_agent_runtime_status
    ON local_agent_runtime_records (status);
