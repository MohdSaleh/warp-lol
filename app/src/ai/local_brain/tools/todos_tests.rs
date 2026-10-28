use serde_json::json;
use warp_multi_agent_api as api;

use super::*;

#[test]
fn create_todos_decodes_to_the_structured_warp_message() {
    let update = decode_update(json!({
        "operation": "create",
        "todos": [{
            "id": "T1",
            "title": "Inspect the repository",
            "description": "Read the project rules"
        }]
    }))
    .unwrap();

    let Some(api::message::update_todos::Operation::CreateTodoList(create)) = update.operation
    else {
        panic!("expected create todo list operation");
    };
    assert_eq!(create.initial_todos[0].id, "T1");
    assert_eq!(create.initial_todos[0].title, "Inspect the repository");
}

#[test]
fn todo_history_round_trips_as_an_update_todos_tool_call() {
    let update = api::message::UpdateTodos {
        operation: Some(api::message::update_todos::Operation::MarkTodosCompleted(
            api::MarkTodosCompleted {
                todo_ids: vec!["T1".to_string()],
            },
        )),
    };

    let (name, arguments) = encode_history(&update).unwrap();
    let arguments: serde_json::Value = serde_json::from_str(&arguments).unwrap();

    assert_eq!(name, UPDATE_TODOS_NAME);
    assert_eq!(arguments["operation"], "mark_completed");
    assert_eq!(arguments["todo_ids"][0], "T1");
}

#[test]
fn todo_updates_reject_empty_or_blank_items() {
    let empty = decode_update(json!({"operation": "create", "todos": []}));
    assert!(empty.is_err());

    let blank = decode_update(json!({
        "operation": "update_pending",
        "todos": [{"id": " ", "title": "Inspect"}]
    }));
    assert!(blank.is_err());

    let missing_completed_ids = decode_update(json!({
        "operation": "mark_completed",
        "todo_ids": []
    }));
    assert!(missing_completed_ids.is_err());
}
