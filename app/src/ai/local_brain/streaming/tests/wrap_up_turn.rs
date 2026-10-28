use std::sync::{Arc, Mutex};

use axum::extract::Json;
use axum::response::sse::{Event, Sse};
use axum::routing::post;
use axum::Router;
use futures::stream::StreamExt;
use tokio::net::TcpListener;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::local_brain::{generate_output, WRAP_UP_INSTRUCTION};

const SUMMARY_CHUNK: &str =
    r#"{"choices":[{"delta":{"content":"Summary: inspected the project; tests remain."},"finish_reason":"stop"}]}"#;
const TOOL_CALL_CHUNK: &str = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_wrap","type":"function","function":{"name":"run_command","arguments":"{\"command\": \"ls\"}"}}]}}]}"#;

/// Mock provider that captures every request body and always answers with
/// `chunk` followed by `[DONE]`.
pub struct CapturingServer {
    pub addr: std::net::SocketAddr,
    pub requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl CapturingServer {
    pub async fn spawn(chunk: &'static str) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(move |Json(body): Json<serde_json::Value>| {
                captured.lock().unwrap().push(body);
                async move {
                    let (tx, rx) =
                        tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(10);
                    let _ = tx.send(Ok(Event::default().data(chunk))).await;
                    let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;

                    Sse::new(async_stream::stream! {
                        let mut rx = rx;
                        while let Some(msg) = rx.recv().await {
                            yield msg;
                        }
                    })
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self { addr, requests }
    }
}

fn test_params(base_url: String) -> RequestParams {
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

/// A task history that already spent the full 6-model-turn budget: one user
/// root followed by six tool calls, each from its own provider request.
fn exhausted_task() -> api::Task {
    let mut task = api::Task {
        id: "task_1".to_string(),
        ..Default::default()
    };
    task.messages.push(api::Message {
        id: "msg_user_1".to_string(),
        task_id: "task_1".to_string(),
        request_id: "req_0".to_string(),
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query: "Refactor the module".to_string(),
            ..Default::default()
        })),
        ..Default::default()
    });
    for turn in 0..6 {
        task.messages.push(api::Message {
            id: format!("msg_tool_{turn}"),
            task_id: "task_1".to_string(),
            request_id: format!("req_turn_{turn}"),
            message: Some(api::message::Message::ToolCall(api::message::ToolCall {
                tool_call_id: format!("call_{turn}"),
                tool: None,
            })),
            ..Default::default()
        });
    }
    task
}

struct Collected {
    output: String,
    tool_calls: usize,
    saw_finished: bool,
}

async fn collect(mut stream: crate::ai::agent::api::ResponseStream) -> Collected {
    let mut collected = Collected {
        output: String::new(),
        tool_calls: 0,
        saw_finished: false,
    };

    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        match event.r#type {
            Some(api::response_event::Type::Finished(_)) => collected.saw_finished = true,
            Some(api::response_event::Type::ClientActions(actions)) => {
                for action in actions.actions {
                    match action.action {
                        Some(api::client_action::Action::AddMessagesToTask(add)) => {
                            for message in add.messages {
                                match message.message {
                                    Some(api::message::Message::AgentOutput(output)) => {
                                        collected.output.push_str(&output.text)
                                    }
                                    Some(api::message::Message::ToolCall(_)) => {
                                        collected.tool_calls += 1
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(api::client_action::Action::AppendToMessageContent(append)) => {
                            if let Some(api::message::Message::AgentOutput(output)) =
                                append.message.and_then(|message| message.message)
                            {
                                collected.output.push_str(&output.text);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    collected
}

fn assert_wrap_up_request(requests: &[serde_json::Value]) {
    assert_eq!(requests.len(), 1, "only the wrap-up turn should be sent");
    let request = &requests[0];
    assert!(
        request.get("tools").is_none(),
        "tools must be disabled on the wrap-up turn: {request:?}"
    );
    assert!(request.get("tool_choice").is_none());
    let last = request["messages"]
        .as_array()
        .expect("messages array")
        .last()
        .expect("a final message");
    assert_eq!(last["role"], "user");
    assert_eq!(last["content"], WRAP_UP_INSTRUCTION);
}

#[tokio::test]
async fn exhausted_budget_runs_one_text_only_wrap_up_turn() {
    let server = CapturingServer::spawn(SUMMARY_CHUNK).await;
    let mut params = test_params(format!("http://{}", server.addr));
    params.tasks = vec![exhausted_task()];

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let stream = generate_output(params, cancel_rx).await.unwrap();
    let collected = collect(stream).await;

    assert!(collected.saw_finished);
    assert_eq!(collected.tool_calls, 0);
    assert!(
        collected.output.contains("Summary:"),
        "expected the model's wrap-up text, got {:?}",
        collected.output
    );
    assert_wrap_up_request(&server.requests.lock().unwrap());
}

#[tokio::test]
async fn wrap_up_turn_suppresses_tool_calls_the_model_still_returns() {
    let server = CapturingServer::spawn(TOOL_CALL_CHUNK).await;
    let mut params = test_params(format!("http://{}", server.addr));
    params.tasks = vec![exhausted_task()];

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let stream = generate_output(params, cancel_rx).await.unwrap();
    let collected = collect(stream).await;

    assert!(collected.saw_finished);
    assert_eq!(collected.tool_calls, 0, "wrap-up must not emit tool calls");
    assert!(
        collected.output.contains("model turn limit reached"),
        "expected the limit message as a fallback, got {:?}",
        collected.output
    );
    assert_wrap_up_request(&server.requests.lock().unwrap());
}
