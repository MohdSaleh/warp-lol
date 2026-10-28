CREATE TABLE orchestration_router_runs (
    run_id TEXT NOT NULL PRIMARY KEY,
    root_run_id TEXT NOT NULL,
    parent_run_id TEXT,
    conversation_id TEXT NOT NULL,
    runtime_kind TEXT NOT NULL,
    agent_role TEXT,
    lifecycle_state TEXT NOT NULL,
    last_source_sequence INTEGER,
    last_ingest_sequence INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    terminal_reason TEXT
);

CREATE INDEX ix_orchestration_router_runs_root ON orchestration_router_runs (root_run_id);
CREATE INDEX ix_orchestration_router_runs_parent ON orchestration_router_runs (parent_run_id);

CREATE TABLE orchestration_router_edges (
    parent_run_id TEXT NOT NULL,
    child_run_id TEXT NOT NULL,
    PRIMARY KEY (parent_run_id, child_run_id)
);

CREATE TABLE orchestration_router_outbox (
    event_id TEXT NOT NULL PRIMARY KEY,
    run_id TEXT NOT NULL,
    event_payload TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
