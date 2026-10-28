//! End-to-end checks that the harness owns execution of its filesystem tools:
//! a turn whose calls are all harness-owned keeps looping inside one request,
//! while client-owned tools still leave through the action path exactly once.

use std::sync::{Arc, Mutex};

use axum::extract::Json;
use axum::response::sse::{Event, Sse};
use axum::routing::post;
use axum::Router;
use futures::stream::StreamExt;
use serde_json::json;
use tokio::net::TcpListener;
use warp_multi_agent_api as api;

use crate::ai::agent::api::{RequestParams, ResponseStream};
use crate::ai::blocklist::SessionContext;
use crate::ai::local_brain::{generate_output, WRAP_UP_INSTRUCTION};

struct SequencedServer {
    addr: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl SequencedServer {
    /// Answers the Nth provider request with the Nth chunk, repeating the last
    /// chunk if the model asks for more turns than scripted.
    async fn spawn(chunks: Vec<String>) -> Self {
        let requests: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let chunks = Arc::new(chunks);

        let app = Router::new().route(
            "/chat/completions",
            post(move |Json(body): Json<serde_json::Value>| {
                let captured = captured.clone();
                let chunks = chunks.clone();
                async move {
                    let index = {
                        let mut guard = captured.lock().unwrap();
                        guard.push(body);
                        guard.len() - 1
                    };
                    let chunk = chunks
                        .get(index)
                        .or_else(|| chunks.last())
                        .cloned()
                        .unwrap_or_default();

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

fn tool_call(name: &str, id: &str, index: usize, arguments: serde_json::Value) -> serde_json::Value {
    json!({
        "index": index,
        "id": id,
        "type": "function",
        "function": { "name": name, "arguments": arguments.to_string() },
    })
}

fn tool_call_chunk(calls: Vec<serde_json::Value>) -> String {
    json!({
        "choices": [{
            "delta": { "tool_calls": calls },
            "finish_reason": "tool_calls",
        }],
    })
    .to_string()
}

fn text_chunk(text: &str) -> String {
    json!({
        "choices": [{
            "delta": { "content": text },
            "finish_reason": "stop",
        }],
    })
    .to_string()
}

fn read_chunk(id: &str, file: &str, offset: usize) -> String {
    tool_call_chunk(vec![tool_call(
        "read_files",
        id,
        0,
        json!({ "paths": [file], "offset": offset }),
    )])
}

fn test_params(base_url: String, working_directory: &std::path::Path) -> RequestParams {
    let mut params = RequestParams::new_for_test();
    params.model = "cfg-1".into();
    params.session_context = SessionContext::new_with_working_directory_for_test(
        working_directory.to_string_lossy().to_string(),
    );
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

fn user_task(query: &str) -> api::Task {
    let mut task = api::Task {
        id: "task_1".to_string(),
        ..Default::default()
    };
    task.messages.push(api::Message {
        id: "msg_user_1".to_string(),
        task_id: "task_1".to_string(),
        request_id: "req_0".to_string(),
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query: query.to_string(),
            ..Default::default()
        })),
        ..Default::default()
    });
    task
}

fn scratch_directory(name: &str, file_contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "warp-harness-{name}-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("notes.txt"), file_contents).unwrap();
    dir
}

struct Collected {
    output: String,
    client_tool_calls: usize,
    server_tool_calls: usize,
    server_tool_results: usize,
    saw_finished: bool,
}

impl Collected {
    fn new() -> Self {
        Self {
            output: String::new(),
            client_tool_calls: 0,
            server_tool_calls: 0,
            server_tool_results: 0,
            saw_finished: false,
        }
    }

    fn record_message(&mut self, message: api::Message) {
        match message.message {
            Some(api::message::Message::AgentOutput(output)) => {
                self.output.push_str(&output.text)
            }
            Some(api::message::Message::ToolCall(call)) => {
                if matches!(call.tool, Some(api::message::tool_call::Tool::Server(_))) {
                    self.server_tool_calls += 1;
                } else {
                    self.client_tool_calls += 1;
                }
            }
            Some(api::message::Message::ToolCallResult(result)) => {
                if matches!(
                    result.result,
                    Some(api::message::tool_call_result::Result::Server(_))
                ) {
                    self.server_tool_results += 1;
                }
            }
            _ => {}
        }
    }
}

async fn collect(mut stream: ResponseStream) -> Collected {
    let mut collected = Collected::new();

    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        match event.r#type {
            Some(api::response_event::Type::Finished(_)) => collected.saw_finished = true,
            Some(api::response_event::Type::ClientActions(actions)) => {
                for action in actions.actions {
                    match action.action {
                        Some(api::client_action::Action::AddMessagesToTask(add)) => {
                            for message in add.messages {
                                collected.record_message(message);
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

fn request_messages(request: &serde_json::Value) -> &Vec<serde_json::Value> {
    request["messages"].as_array().expect("messages array")
}

#[tokio::test]
async fn harness_executes_fs_tools_inside_the_request_loop() {
    let dir = scratch_directory("fs-in-process", "alpha beta\ngamma delta\n");
    let server = SequencedServer::spawn(vec![
        read_chunk("call_read", "notes.txt", 1),
        text_chunk("Read the notes file."),
    ])
    .await;
    let mut params = test_params(format!("http://{}", server.addr), &dir);
    params.tasks = vec![user_task("Inspect the notes")];

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let stream = generate_output(params, cancel_rx).await.unwrap();
    let collected = collect(stream).await;

    let requests = server.requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        2,
        "the fs call must be executed in-process, not handed back to the client"
    );
    assert_eq!(collected.client_tool_calls, 0);
    assert_eq!(collected.server_tool_calls, 1, "call replay record");
    assert_eq!(collected.server_tool_results, 1, "result replay record");
    assert!(collected.saw_finished);
    assert!(collected.output.contains("Read the notes file."));

    let messages = request_messages(&requests[1]);
    let tool_message = messages
        .iter()
        .find(|message| message["role"] == "tool")
        .expect("the observation must be in the next provider turn");
    assert!(
        tool_message["content"]
            .as_str()
            .unwrap()
            .contains("alpha beta"),
        "the harness must feed real file contents back: {tool_message:?}"
    );
    let assistant_call = messages
        .iter()
        .find(|message| message["role"] == "assistant" && message.get("tool_calls").is_some())
        .expect("assistant tool call in the next provider turn");
    assert_eq!(
        assistant_call["tool_calls"][0]["function"]["name"],
        "read_files"
    );

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn client_owned_tools_stay_on_the_client_action_path() {
    let dir = scratch_directory("client-tool", "unused\n");
    let server = SequencedServer::spawn(vec![tool_call_chunk(vec![tool_call(
        "run_command",
        "call_shell",
        0,
        json!({ "command": "echo hi" }),
    )])])
    .await;
    let mut params = test_params(format!("http://{}", server.addr), &dir);
    params.tasks = vec![user_task("Run a command")];

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let stream = generate_output(params, cancel_rx).await.unwrap();
    let collected = collect(stream).await;

    assert_eq!(server.requests.lock().unwrap().len(), 1);
    assert_eq!(collected.client_tool_calls, 1);
    assert_eq!(collected.server_tool_calls, 0);
    assert!(collected.saw_finished);

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn mixed_batch_executes_owned_calls_then_hands_back_client_calls() {
    let dir = scratch_directory("mixed-batch", "mixed content\n");
    let server = SequencedServer::spawn(vec![tool_call_chunk(vec![
        tool_call("read_files", "call_read", 0, json!({ "paths": ["notes.txt"] })),
        tool_call("run_command", "call_shell", 1, json!({ "command": "echo hi" })),
    ])])
    .await;
    let mut params = test_params(format!("http://{}", server.addr), &dir);
    params.tasks = vec![user_task("Read and run")];

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let stream = generate_output(params, cancel_rx).await.unwrap();
    let collected = collect(stream).await;

    assert_eq!(
        server.requests.lock().unwrap().len(),
        1,
        "the client call ends the stream; it must not be re-requested"
    );
    assert_eq!(collected.client_tool_calls, 1);
    assert_eq!(collected.server_tool_calls, 1);
    assert_eq!(collected.server_tool_results, 1);
    assert!(collected.saw_finished);

    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn in_process_turns_count_toward_the_model_turn_budget() {
    let dir = scratch_directory(
        "turn-budget",
        "1 one\n2 two\n3 three\n4 four\n5 five\n6 six\n7 seven\n8 eight\n",
    );
    let mut chunks: Vec<String> = (1..=6)
        .map(|offset| read_chunk(&format!("call_read_{offset}"), "notes.txt", offset))
        .collect();
    chunks.push(text_chunk("Summary after six turns."));
    let server = SequencedServer::spawn(chunks).await;
    let mut params = test_params(format!("http://{}", server.addr), &dir);
    params.tasks = vec![user_task("Read every line")];

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let stream = generate_output(params, cancel_rx).await.unwrap();
    let collected = collect(stream).await;

    let requests = server.requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        7,
        "six tool turns, then one text-only wrap-up turn"
    );
    let wrap_up = &requests[6];
    assert!(
        wrap_up.get("tools").is_none(),
        "the wrap-up turn must be text-only: {wrap_up:?}"
    );
    let last = request_messages(wrap_up).last().expect("a final message");
    assert_eq!(last["content"], WRAP_UP_INSTRUCTION);
    assert!(collected.saw_finished);
    assert!(collected.output.contains("Summary after six turns."));
    assert_eq!(collected.client_tool_calls, 0);

    std::fs::remove_dir_all(dir).ok();
}
