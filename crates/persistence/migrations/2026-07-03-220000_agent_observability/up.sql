CREATE TABLE agent_usage_entries (
    usage_entry_id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    logical_request_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,

    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    route_type TEXT NOT NULL,

    source TEXT NOT NULL,
    status TEXT NOT NULL,

    token_usage_json TEXT NOT NULL,
    provider_breakdown_json TEXT NOT NULL,

    provider_request_id TEXT,
    supersedes_entry_id TEXT,

    observed_at TIMESTAMP NOT NULL
);

CREATE TABLE agent_cost_entries (
    cost_entry_id TEXT PRIMARY KEY NOT NULL,
    usage_entry_id TEXT NOT NULL UNIQUE,

    amount TEXT,
    currency TEXT,

    classification TEXT NOT NULL,
    pricing_catalog_version TEXT,
    calculation_json TEXT,

    created_at TIMESTAMP NOT NULL
);

CREATE TABLE agent_observation_events (
    event_id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    logical_request_id TEXT,
    attempt_id TEXT,
    
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,

    occurred_at TIMESTAMP NOT NULL
);

CREATE TABLE agent_observation_rollups (
    rollup_id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    
    scope TEXT NOT NULL,
    token_usage_json TEXT NOT NULL,
    cost_json TEXT NOT NULL,
    
    attempts INTEGER NOT NULL,
    retries INTEGER NOT NULL,
    fallbacks INTEGER NOT NULL,
    
    authoritative_entries INTEGER NOT NULL,
    estimated_entries INTEGER NOT NULL,
    
    created_at TIMESTAMP NOT NULL
);
