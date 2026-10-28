use warp_multi_agent_api as api;

pub fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn api_message_with_id(
    id: &str,
    task_id: &str,
    request_id: &str,
    message: api::message::Message,
) -> api::Message {
    api::Message {
        id: id.to_string(),
        task_id: task_id.to_string(),
        request_id: request_id.to_string(),
        timestamp: None,
        server_message_data: String::new(),
        citations: vec![],
        fetched_memories: vec![],
        message: Some(message),
    }
}

pub fn api_message(
    task_id: &str,
    request_id: &str,
    message: api::message::Message,
) -> api::Message {
    api_message_with_id(&new_uuid(), task_id, request_id, message)
}

pub fn agent_reasoning_msg(
    task_id: &str,
    request_id: &str,
    id: &str,
    reasoning: &str,
    finished_duration: Option<std::time::Duration>,
) -> api::Message {
    api_message_with_id(
        id,
        task_id,
        request_id,
        api::message::Message::AgentReasoning(api::message::AgentReasoning {
            reasoning: reasoning.to_string(),
            finished_duration: finished_duration.map(|d| prost_types::Duration {
                seconds: d.as_secs() as i64,
                nanos: d.subsec_nanos() as i32,
            }),
        }),
    )
}

pub fn agent_output_msg(task_id: &str, request_id: &str, id: &str, text: &str) -> api::Message {
    api_message_with_id(
        id,
        task_id,
        request_id,
        api::message::Message::AgentOutput(api::message::AgentOutput {
            text: text.to_string(),
        }),
    )
}

pub fn output_message(task_id: &str, request_id: &str, text: &str) -> api::Message {
    api_message(
        task_id,
        request_id,
        api::message::Message::AgentOutput(api::message::AgentOutput {
            text: text.to_string(),
        }),
    )
}

pub fn tool_call_message(
    task_id: &str,
    request_id: &str,
    tool_call_id: String,
    tool: api::message::tool_call::Tool,
) -> api::Message {
    api_message(
        task_id,
        request_id,
        api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id,
            tool: Some(tool),
        }),
    )
}

pub fn update_todos_message(
    task_id: &str,
    request_id: &str,
    update: api::message::UpdateTodos,
) -> api::Message {
    api_message(
        task_id,
        request_id,
        api::message::Message::UpdateTodos(update),
    )
}

pub fn append_content_event(
    task_id: String,
    message: api::Message,
    mask_path: &str,
) -> api::ResponseEvent {
    client_actions(vec![api::ClientAction {
        action: Some(api::client_action::Action::AppendToMessageContent(
            api::client_action::AppendToMessageContent {
                task_id,
                message: Some(message),
                mask: Some(prost_types::FieldMask {
                    paths: vec![mask_path.to_string()],
                }),
            },
        )),
    }])
}

pub fn update_message_event(
    task_id: String,
    message: api::Message,
    mask_path: &str,
) -> api::ResponseEvent {
    client_actions(vec![api::ClientAction {
        action: Some(api::client_action::Action::UpdateTaskMessage(
            api::client_action::UpdateTaskMessage {
                task_id,
                message: Some(message),
                mask: Some(prost_types::FieldMask {
                    paths: vec![mask_path.to_string()],
                }),
            },
        )),
    }])
}

pub fn init_event(
    conversation_id: String,
    request_id: String,
    run_id: String,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Init(
            api::response_event::StreamInit {
                conversation_id,
                request_id,
                run_id,
            },
        )),
    }
}

pub fn create_task_event(task_id: String, description: String) -> api::ResponseEvent {
    client_actions(vec![api::ClientAction {
        action: Some(api::client_action::Action::CreateTask(
            api::client_action::CreateTask {
                task: Some(api::Task {
                    id: task_id,
                    description,
                    dependencies: None,
                    messages: vec![],
                    summary: String::new(),
                    server_data: String::new(),
                }),
            },
        )),
    }])
}

pub fn add_messages_event(task_id: String, messages: Vec<api::Message>) -> api::ResponseEvent {
    client_actions(vec![api::ClientAction {
        action: Some(api::client_action::Action::AddMessagesToTask(
            api::client_action::AddMessagesToTask { task_id, messages },
        )),
    }])
}

pub fn client_actions(actions: Vec<api::ClientAction>) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions { actions },
        )),
    }
}

pub fn finished_event(
    model_id: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(
            api::response_event::StreamFinished {
                token_usage: vec![api::response_event::stream_finished::TokenUsage {
                    model_id,
                    total_input: prompt_tokens,
                    output: completion_tokens,
                    ..Default::default()
                }],
                should_refresh_model_config: false,
                request_cost: None,
                conversation_usage_metadata: None,
                reason: Some(api::response_event::stream_finished::Reason::Done(
                    api::response_event::stream_finished::Done {},
                )),
            },
        )),
    }
}
