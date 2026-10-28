use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::response::sse::{Event, Sse};
use axum::routing::post;
use axum::Router;
use futures::stream::StreamExt;
use tokio::net::TcpListener;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::AIAgentInput;
use crate::ai::local_brain::generate_output;

pub struct MockServer {
    pub addr: std::net::SocketAddr,
    pub request_count: Arc<AtomicUsize>,
}

impl MockServer {
    pub async fn spawn() -> Self {
        let request_count = Arc::new(AtomicUsize::new(0));
        let count_clone = request_count.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(move || {
                count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(10);

                    let tool_call = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","type":"function","function":{"name":"run_command","arguments":"{\"command\": \"ls\"}"}}]}}]}"#;
                    let _ = tx.send(Ok(Event::default().data(tool_call))).await;
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

        Self {
            addr,
            request_count,
        }
    }
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

#[tokio::test]
async fn repeated_tool_loop() {
    let server = MockServer::spawn().await;
    let base_url = format!("http://{}", server.addr);

    let mut params = params_with_model(base_url);

    let mut task = api::Task::default();
    task.messages.push(api::Message {
        id: "msg_user_123".to_string(),
        task_id: "task_1".to_string(),
        request_id: "req_1".to_string(),
        timestamp: None,
        server_message_data: String::new(),
        citations: vec![],
        fetched_memories: vec![],
        message: Some(api::message::Message::UserQuery(api::message::UserQuery {
            query: "Do this".to_string(),
            ..Default::default()
        })),
    });

    let mut limit_message_emitted = false;

    for _ in 0..10 {
        params.tasks = vec![task.clone()];

        let (cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
        let mut stream = generate_output(params.clone(), cancel_rx).await.unwrap();

        let mut got_tool = false;

        while let Some(Ok(event)) = stream.next().await {
            if let Some(api::response_event::Type::ClientActions(c)) = event.r#type {
                for action in c.actions {
                    if let Some(api::client_action::Action::AddMessagesToTask(a)) = action.action {
                        for msg in a.messages {
                            println!("GOT MESSAGE: {:?}", msg.message);
                            task.messages.push(msg.clone());
                            if let Some(api::message::Message::ToolCall(_)) = msg.message {
                                got_tool = true;
                            } else if let Some(api::message::Message::AgentOutput(n)) = &msg.message
                            {
                                println!("AGENT OUTPUT: {}", n.text);
                                if n.text.contains("repeated the same tool call") {
                                    limit_message_emitted = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if limit_message_emitted {
            break;
        }

        if !got_tool {
            break;
        }
    }

    assert!(limit_message_emitted, "Limit message should be emitted");
    assert_eq!(
        server.request_count.load(Ordering::SeqCst),
        3,
        "Provider request count should be exactly 3"
    );
}
