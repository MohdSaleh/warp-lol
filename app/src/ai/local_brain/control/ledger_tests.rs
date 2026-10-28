use super::*;

fn task(messages: Vec<api::Message>) -> api::Task {
    api::Task {
        id: "task".to_string(),
        messages,
        ..Default::default()
    }
}

fn user_message(id: &str, query: &str) -> api::Message {
    api::Message {
        id: id.to_string(),
        task_id: "task".to_string(),
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query: query.to_string(),
            ..Default::default()
        })),
        ..Default::default()
    }
}

fn tool_call_message(id: &str, request_id: &str) -> api::Message {
    api::Message {
        id: format!("message-{id}"),
        task_id: "task".to_string(),
        request_id: request_id.to_string(),
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: id.to_string(),
            tool: None,
        })),
        ..Default::default()
    }
}

#[test]
fn new_user_query_starts_empty_pending_ledger() {
    let tasks = vec![task(vec![
        user_message("old-root", "same question"),
        tool_call_message("old-call", "old-request"),
    ])];

    let ledger = Ledger::for_request(&tasks, Some("same question"));

    assert_eq!(ledger.root_user_message_id, PENDING_ROOT_USER_MESSAGE_ID);
    assert_eq!(ledger.model_turns, 0);
    assert_eq!(ledger.tool_calls, 0);
}

#[test]
fn persisted_user_query_with_no_turns_is_current_root() {
    let tasks = vec![task(vec![user_message("current-root", "question")])];

    let ledger = Ledger::for_request(&tasks, Some("question"));

    assert_eq!(ledger.root_user_message_id, "current-root");
    assert_eq!(ledger.model_turns, 0);
    assert_eq!(ledger.tool_calls, 0);
}

#[test]
fn counts_three_turns_for_current_root() {
    let tasks = vec![task(vec![
        user_message("root", "question"),
        tool_call_message("call-1", "request-1"),
        tool_call_message("call-2", "request-2"),
        tool_call_message("call-3", "request-3"),
    ])];

    let ledger = Ledger::for_request(&tasks, None);

    assert_eq!(ledger.root_user_message_id, "root");
    assert_eq!(ledger.model_turns, 3);
    assert_eq!(ledger.tool_calls, 3);
    assert_eq!(ledger.recent_tool_calls.len(), 3);
}

#[test]
fn older_root_turns_do_not_count() {
    let tasks = vec![task(vec![
        user_message("old-root", "old question"),
        tool_call_message("old-call-1", "old-request-1"),
        tool_call_message("old-call-2", "old-request-2"),
        user_message("current-root", "current question"),
        tool_call_message("current-call", "current-request"),
    ])];

    let ledger = Ledger::for_request(&tasks, None);

    assert_eq!(ledger.root_user_message_id, "current-root");
    assert_eq!(ledger.model_turns, 1);
    assert_eq!(ledger.tool_calls, 1);
    assert_eq!(ledger.recent_tool_calls[0].call_id, "current-call");
}

#[test]
fn multiple_calls_from_one_provider_request_are_one_turn() {
    let tasks = vec![task(vec![
        user_message("root", "question"),
        tool_call_message("call-1", "request"),
        tool_call_message("call-2", "request"),
    ])];

    let ledger = Ledger::for_request(&tasks, None);

    assert_eq!(ledger.model_turns, 1);
    assert_eq!(ledger.tool_calls, 2);
}

#[test]
fn to_runtime_record_maps_ledger_fields_correctly() {
    let ledger = Ledger {
        model_turns: 2,
        tool_calls: 3,
        status: LocalAgentRunStatus::Active,
        ..Default::default()
    };

    let record = ledger.to_runtime_record("conv-123", "run-456");

    assert_eq!(record.conversation_id, "conv-123");
    assert_eq!(record.run_id, "run-456");
    assert_eq!(record.status, "Active");
    assert!(record.ledger_json.contains("\"model_turns\":2"));
}
