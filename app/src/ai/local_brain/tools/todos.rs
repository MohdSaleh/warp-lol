use serde::Deserialize;
use serde_json::{json, Value};
use warp_multi_agent_api as api;

use super::errors::ToolArgumentError;
use super::registry::ProviderNeutralToolCall;

pub const UPDATE_TODOS_NAME: &str = "update_todos";

/// Observation recorded for a todo update, which changes UI state rather than
/// producing tool output. Shared so the harness and history reconstruction
/// answer the provider call identically.
pub const TODO_UPDATE_RESULT: &str = r#"{"status":"success","message":"Todo list updated"}"#;

pub fn schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": UPDATE_TODOS_NAME,
            "description": "Update Warp's visible task checklist. Use create once for a new plan, update_pending when pending items change, and mark_completed after work is verified. This is a UI state update and may accompany one execution tool call.",
            "parameters": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["create", "update_pending", "mark_completed"]
                    },
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "string", "minLength": 1},
                                "title": {"type": "string", "minLength": 1},
                                "description": {"type": "string"}
                            },
                            "required": ["id", "title"]
                        },
                        "minItems": 1
                    },
                    "todo_ids": {
                        "type": "array",
                        "items": {"type": "string", "minLength": 1},
                        "minItems": 1
                    }
                },
                "required": ["operation"]
            }
        }
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Create,
    UpdatePending,
    MarkCompleted,
}

#[derive(Deserialize)]
struct Todo {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct Args {
    operation: Operation,
    #[serde(default)]
    todos: Vec<Todo>,
    #[serde(default)]
    todo_ids: Vec<String>,
}

pub fn decode_update(args: Value) -> Result<api::message::UpdateTodos, ToolArgumentError> {
    let args: Args = serde_json::from_value(args)
        .map_err(|error| ToolArgumentError::InvalidSchema(error.to_string()))?;

    let operation = match args.operation {
        Operation::Create => {
            validate_todos(&args.todos)?;
            api::message::update_todos::Operation::CreateTodoList(api::CreateTodoList {
                initial_todos: args.todos.into_iter().map(todo_item).collect(),
            })
        }
        Operation::UpdatePending => {
            validate_todos(&args.todos)?;
            api::message::update_todos::Operation::UpdatePendingTodos(api::UpdatePendingTodos {
                updated_pending_todos: args.todos.into_iter().map(todo_item).collect(),
            })
        }
        Operation::MarkCompleted => {
            if args.todo_ids.is_empty() {
                return Err(ToolArgumentError::MissingField("todo_ids"));
            }
            api::message::update_todos::Operation::MarkTodosCompleted(api::MarkTodosCompleted {
                todo_ids: args.todo_ids,
            })
        }
    };

    Ok(api::message::UpdateTodos {
        operation: Some(operation),
    })
}

fn validate_todos(todos: &[Todo]) -> Result<(), ToolArgumentError> {
    if todos.is_empty() {
        return Err(ToolArgumentError::MissingField("todos"));
    }
    if todos.iter().any(|todo| todo.id.trim().is_empty()) {
        return Err(ToolArgumentError::InvalidSchema(
            "todo ids must not be blank".to_string(),
        ));
    }
    if todos.iter().any(|todo| todo.title.trim().is_empty()) {
        return Err(ToolArgumentError::InvalidSchema(
            "todo titles must not be blank".to_string(),
        ));
    }
    Ok(())
}

fn todo_item(todo: Todo) -> api::TodoItem {
    api::TodoItem {
        id: todo.id,
        title: todo.title,
        description: todo.description,
    }
}

pub fn encode_history(update: &api::message::UpdateTodos) -> Option<ProviderNeutralToolCall> {
    use api::message::update_todos::Operation;

    let arguments = match update.operation.as_ref()? {
        Operation::CreateTodoList(create) => json!({
            "operation": "create",
            "todos": create.initial_todos.iter().map(todo_json).collect::<Vec<_>>(),
        }),
        Operation::UpdatePendingTodos(update) => json!({
            "operation": "update_pending",
            "todos": update.updated_pending_todos.iter().map(todo_json).collect::<Vec<_>>(),
        }),
        Operation::MarkTodosCompleted(completed) => json!({
            "operation": "mark_completed",
            "todo_ids": completed.todo_ids,
        }),
    };

    Some((UPDATE_TODOS_NAME.to_string(), arguments.to_string()))
}

fn todo_json(todo: &api::TodoItem) -> Value {
    json!({
        "id": todo.id,
        "title": todo.title,
        "description": todo.description,
    })
}

#[cfg(test)]
#[path = "todos_tests.rs"]
mod tests;
