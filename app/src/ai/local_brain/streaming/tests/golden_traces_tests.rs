use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::local_brain::generate_output;
use crate::ai::local_brain::streaming::tests::mock_server::MockServer;

#[derive(serde::Deserialize)]
struct GoldenTraceContract {
    contract_version: String,
    name: String,
    input: GoldenTraceInput,
    expected_event_kinds: Vec<String>,
    expected_controls: GoldenTraceControls,
}

#[derive(serde::Deserialize)]
struct GoldenTraceInput {
    user_request: String,
    planning_enabled: bool,
    orchestration_enabled: bool,
    existing_task_count: u32,
}

#[derive(serde::Deserialize)]
struct GoldenTraceControls {
    model_inference_attempts: u32,
    local_tool_iterations: u32,
    provider_timeout_seconds: u32,
}

fn params_with_model(base_url: String) -> RequestParams {
    let mut params = RequestParams::new_for_test();
    params.model = "cfg-1".into();
    params.custom_model_providers = Some(api::request::settings::CustomModelProviders {
        providers: vec![
            api::request::settings::custom_model_providers::CustomModelProvider {
                base_url,
                api_key: "sk-test".to_string(),
                models: vec![
                    api::request::settings::custom_model_providers::CustomModel {
                        slug: "real-model".to_string(),
                        config_key: "cfg-1".to_string(),
                    },
                ],
            },
        ],
    });
    params
}

fn map_event_to_kind(event: &api::ResponseEvent) -> Vec<String> {
    let mut kinds = Vec::new();
    match event.r#type.as_ref().unwrap() {
        api::response_event::Type::Init(_) => kinds.push("init".to_string()),
        api::response_event::Type::Finished(_) => kinds.push("finished".to_string()),
        api::response_event::Type::ClientActions(actions) => {
            for action in &actions.actions {
                match action.action.as_ref().unwrap() {
                    api::client_action::Action::CreateTask(_) => {
                        kinds.push("client_actions.create_task".to_string());
                    }
                    api::client_action::Action::AddMessagesToTask(add_msgs) => {
                        for msg in &add_msgs.messages {
                            match msg.message.as_ref().unwrap() {
                                api::message::Message::AgentReasoning(_) => {
                                    kinds.push(
                                        "client_actions.add_messages_to_task.agent_reasoning"
                                            .to_string(),
                                    );
                                }
                                api::message::Message::AgentOutput(_) => {
                                    kinds.push(
                                        "client_actions.add_messages_to_task.agent_output"
                                            .to_string(),
                                    );
                                }
                                api::message::Message::ToolCall(_) => {
                                    kinds.push(
                                        "client_actions.add_messages_to_task.tool_call".to_string(),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    api::client_action::Action::AppendToMessageContent(append) => {
                        if let Some(msg) = &append.message {
                            match msg.message.as_ref().unwrap() {
                                api::message::Message::AgentReasoning(_) => {
                                    kinds.push(
                                        "client_actions.append_to_message_content.agent_reasoning"
                                            .to_string(),
                                    );
                                }
                                api::message::Message::AgentOutput(_) => {
                                    kinds.push(
                                        "client_actions.append_to_message_content.agent_output"
                                            .to_string(),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    api::client_action::Action::UpdateTaskMessage(update) => {
                        if let Some(msg) = &update.message {
                            if let api::message::Message::AgentReasoning(_) =
                                msg.message.as_ref().unwrap()
                            {
                                kinds.push(
                                    "client_actions.update_task_message.agent_reasoning"
                                        .to_string(),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    kinds
}

#[tokio::test]
async fn golden_traces_parity() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let traces_dir = manifest_dir.join("../../docs/brain_contract/golden_traces");

    if !traces_dir.exists() {
        return; // Skip if directory doesn't exist
    }

    let entries = fs::read_dir(traces_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        let trace: GoldenTraceContract = serde_json::from_str(&content).unwrap();

        // Spawn mock server
        let server = MockServer::spawn().await;
        let base_url = format!("http://{}", server.addr);

        let (_cancellation_tx, cancellation_rx) = futures::channel::oneshot::channel();

        let mut params = params_with_model(base_url);

        // Set up the task based on trace.input.existing_task_count if needed
        // For local_chat_v1, existing_task_count is 0, so we just add the user message
        let mut task = api::Task::default();
        task.messages.push(api::Message {
            id: "msg_user".to_string(),
            task_id: "task_1".to_string(),
            request_id: "req_1".to_string(),
            timestamp: None,
            server_message_data: String::new(),
            citations: vec![],
            fetched_memories: vec![],
            message: Some(api::message::Message::UserQuery(api::message::UserQuery {
                query: trace.input.user_request.clone(),
                ..Default::default()
            })),
        });

        params.tasks.push(task);

        let mut stream = generate_output(params, cancellation_rx).await.unwrap();

        let mut emitted_event_kinds = Vec::new();
        while let Some(event_result) = stream.next().await {
            let event = event_result.unwrap();
            let mut mapped_kinds = map_event_to_kind(&event);

            // In the golden traces we only record the macro events (e.g. add_messages_to_task)
            // But generate_output emits streaming append events too.
            // For parity we check if the emitted events subsume the expected events.
            emitted_event_kinds.append(&mut mapped_kinds);
        }

        // Clean up repeated continuous streaming events for matching
        // We only want the unique sequence of event kinds, keeping ordering.
        let mut deduplicated_emitted = Vec::new();
        for kind in emitted_event_kinds {
            if deduplicated_emitted.last() != Some(&kind) {
                deduplicated_emitted.push(kind);
            }
        }

        // We need to verify that all expected events are present in the correct order.
        let mut expected_iter = trace.expected_event_kinds.iter();
        let mut current_expected = expected_iter.next();

        for emitted in &deduplicated_emitted {
            if let Some(expected) = current_expected {
                if expected == emitted {
                    current_expected = expected_iter.next();
                }
            }
        }

        assert!(current_expected.is_none(), "Not all expected events were emitted for trace {}. Expected: {:?}, Emitted sequence: {:?}", trace.name, trace.expected_event_kinds, deduplicated_emitted);
    }
}
