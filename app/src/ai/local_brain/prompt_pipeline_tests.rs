use std::collections::HashMap;
use std::sync::Arc;

use ai::agent::action_result::{
    AIAgentActionResultType, AskUserQuestionAnswerItem, AskUserQuestionResult,
};

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::task::TaskId;
use crate::ai::agent::{
    AIAgentActionId, AIAgentActionResult, AIAgentContext, AIAgentInput, UserQueryMode,
};
use crate::ai::local_brain::{history, prompt, tools};

fn create_user_query(text: &str) -> AIAgentInput {
    AIAgentInput::UserQuery {
        query: text.to_string(),
        context: Arc::new([]),
        static_query_type: None,
        referenced_attachments: HashMap::new(),
        user_query_mode: UserQueryMode::Normal,
        running_command: None,
        intended_agent: None,
    }
}

fn create_params_with_input(input: Vec<AIAgentInput>) -> RequestParams {
    let mut params = RequestParams::new_for_test();
    params.input = input;
    params
}

fn create_init_project_rules() -> AIAgentInput {
    AIAgentInput::InitProjectRules {
        context: Arc::new([]),
        display_query: Some("/init".to_string()),
    }
}

#[test]
fn test_first_prompt_reaches_build_messages_unchanged() {
    let query = "Hello World".to_string();
    let params = create_params_with_input(vec![create_user_query(&query)]);
    let system_prompt = prompt::system_prompt(&params);
    let built = history::build_messages(&params, &system_prompt);

    assert!(built
        .messages
        .iter()
        .any(|m| m.role == "user" && m.content == Some(query.clone())));
}

#[test]
fn test_new_session_cannot_overwrite_submitted_prompt() {
    let params_new = RequestParams::new_for_test();
    let params_with_query = create_params_with_input(vec![create_user_query("test query")]);

    let built_new = history::build_messages(&params_new, "sys");
    let built_query = history::build_messages(&params_with_query, "sys");

    assert!(!built_new.messages.iter().any(|m| m.role == "user"));
    assert!(built_query
        .messages
        .iter()
        .any(|m| m.role == "user" && m.content == Some("test query".to_string())));
}

#[test]
fn test_empty_query_does_not_produce_user_message() {
    let params = create_params_with_input(vec![create_user_query("   ")]);
    let built = history::build_messages(&params, "sys");

    assert!(!built.messages.iter().any(|m| m.role == "user"));
}

#[test]
fn test_long_prompt_survives_build_messages() {
    let long_prompt = "A".repeat(50_000);
    let params = create_params_with_input(vec![create_user_query(&long_prompt)]);
    let built = history::build_messages(&params, "sys");

    assert!(built
        .messages
        .iter()
        .any(|m| m.role == "user" && m.content == Some(long_prompt.clone())));
}

#[test]
fn test_latest_user_text_extracts_from_input() {
    let params = create_params_with_input(vec![create_user_query("expected query")]);
    assert_eq!(
        history::latest_user_text(&params).unwrap(),
        "expected query"
    );
}

#[test]
fn test_init_project_rules_becomes_an_explicit_model_visible_request() {
    let params = create_params_with_input(vec![create_init_project_rules()]);
    let built = history::build_messages(&params, "sys");
    let user_message = built
        .messages
        .iter()
        .find(|message| message.role == "user")
        .and_then(|message| message.content.as_deref())
        .expect("init should produce a model-visible user message");

    assert!(user_message.contains("Initialize this project now"));
    assert!(user_message.contains("Do not ask the user to restate the task"));
    assert_ne!(user_message.trim(), "/init");
}

#[test]
fn test_selected_question_answer_reaches_the_follow_up_model_request() {
    let result = AIAgentActionResult {
        id: AIAgentActionId::from("question-call".to_string()),
        task_id: TaskId::new("question-task".to_string()),
        result: AIAgentActionResultType::AskUserQuestion(AskUserQuestionResult::Success {
            answers: vec![AskUserQuestionAnswerItem::Answered {
                question_id: "q0".to_string(),
                selected_options: vec!["PostgreSQL".to_string()],
                other_text: "Use the managed cluster".to_string(),
            }],
        }),
    };
    let params = create_params_with_input(vec![AIAgentInput::ActionResult {
        result,
        context: Arc::new([]),
    }]);

    let built = history::build_messages(&params, "sys");
    let tool_result = built
        .messages
        .iter()
        .find(|message| message.role == "tool")
        .and_then(|message| message.content.as_deref())
        .expect("selected answer should produce a tool result");

    assert!(tool_result.contains("PostgreSQL"));
    assert!(tool_result.contains("Use the managed cluster"));
    assert!(tool_result.contains("q0"));
    assert!(!tool_result.contains("completed with 1 answer"));
}

#[test]
fn test_todo_tool_is_only_available_when_planning_is_enabled() {
    let mut disabled = RequestParams::new_for_test();
    disabled.planning_enabled = false;
    assert!(!tools::tool_schemas(&disabled)
        .iter()
        .any(|schema| schema["function"]["name"] == "update_todos"));

    let mut enabled = RequestParams::new_for_test();
    enabled.planning_enabled = true;
    assert!(tools::tool_schemas(&enabled)
        .iter()
        .any(|schema| schema["function"]["name"] == "update_todos"));
}

#[test]
fn test_todo_history_reaches_the_follow_up_model_request() {
    let mut params = RequestParams::new_for_test();
    let mut task = warp_multi_agent_api::Task::default();
    task.messages.push(warp_multi_agent_api::Message {
        id: "todo-message".to_string(),
        task_id: "task-1".to_string(),
        request_id: "request-1".to_string(),
        message: Some(warp_multi_agent_api::message::Message::UpdateTodos(
            warp_multi_agent_api::message::UpdateTodos {
                operation: Some(
                    warp_multi_agent_api::message::update_todos::Operation::CreateTodoList(
                        warp_multi_agent_api::CreateTodoList {
                            initial_todos: vec![warp_multi_agent_api::TodoItem {
                                id: "T1".to_string(),
                                title: "Inspect the repository".to_string(),
                                description: "Read project rules".to_string(),
                            }],
                        },
                    ),
                ),
            },
        )),
        ..Default::default()
    });
    params.tasks.push(task);

    let built = history::build_messages(&params, "sys");
    let todo_call = built
        .messages
        .iter()
        .find_map(|message| message.tool_calls.as_ref())
        .and_then(|calls| calls.first())
        .expect("todo history should produce a model-visible tool call");

    assert_eq!(todo_call.function.name, "update_todos");
    assert!(todo_call.function.arguments.contains("T1"));
    assert!(todo_call
        .function
        .arguments
        .contains("Inspect the repository"));
    assert!(built.messages.iter().any(|message| {
        message.role == "tool"
            && message.tool_call_id.as_deref() == Some("todo-message")
            && message
                .content
                .as_deref()
                .is_some_and(|content| content.contains("Todo list updated"))
    }));
}

#[test]
fn test_nonempty_query_always_produces_user_message() {
    let params = create_params_with_input(vec![create_user_query("non-empty")]);
    let built = history::build_messages(&params, "sys");
    assert!(built.messages.iter().any(|m| m.role == "user"));
}

#[test]
fn test_default_greeting_only_when_no_user_task() {
    // No user task and no input -> greeting occurs implicitly (LLM will respond to sys prompt alone)
    let params = RequestParams::new_for_test();
    let built = history::build_messages(&params, "sys");
    assert!(!built.messages.iter().any(|m| m.role == "user"));

    // User task or input -> has user message
    let params = create_params_with_input(vec![create_user_query("hi")]);
    let built = history::build_messages(&params, "sys");
    assert!(built.messages.iter().any(|m| m.role == "user"));
}
