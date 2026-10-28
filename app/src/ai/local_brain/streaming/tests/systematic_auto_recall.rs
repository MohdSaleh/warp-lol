use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use futures::stream::StreamExt;
use tokio::net::TcpListener;
use warp_multi_agent_api as api;

use crate::ai::agent::api::RequestParams;
use crate::ai::local_brain::generate_output;

pub struct FlakyServer {
    pub addr: std::net::SocketAddr,
    pub request_count: Arc<AtomicUsize>,
}

impl FlakyServer {
    pub async fn spawn(fail_count: usize) -> Self {
        let request_count = Arc::new(AtomicUsize::new(0));
        let count_clone = request_count.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(move || {
                let current = count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if current < fail_count {
                        // Return 500 on earlier attempts
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Temporary Server Error").into_response();
                    }

                    // Succeed on subsequent attempts
                    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(10);

                    let chunk = r#"{"choices":[{"delta":{"content":"Recovered!"},"finish_reason":"stop"}]}"#;
                    let _ = tx.send(Ok(Event::default().data(chunk))).await;
                    let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;

                    Sse::new(async_stream::stream! {
                        let mut rx = rx;
                        while let Some(msg) = rx.recv().await {
                            yield msg;
                        }
                    })
                    .into_response()
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
                        slug: "test-model".to_string(),
                        config_key: "cfg-1".to_string(),
                    },
                ],
            },
        ],
    });

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
            query: "Hello".to_string(),
            ..Default::default()
        })),
    });
    params.tasks = vec![task];
    params
}

#[tokio::test]
async fn systematic_auto_recall_recovers_after_server_errors() {
    // Fail first 2 attempts with 500, succeed on attempt 3
    let server = FlakyServer::spawn(2).await;
    let base_url = format!("http://{}", server.addr);
    let params = test_params(base_url);

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let mut stream = generate_output(params, cancel_rx)
        .await
        .expect("Should create response stream");

    let mut saw_output = false;
    while let Some(event_res) = stream.next().await {
        if let Ok(event) = event_res {
            if let Some(api::response_event::Type::ClientActions(client_actions)) = event.r#type {
                for ca in client_actions.actions {
                    if let Some(api::client_action::Action::AppendToMessageContent(append)) = ca.action {
                        if let Some(msg) = append.message {
                            if let Some(api::message::Message::AgentOutput(out)) = msg.message {
                                if out.text.contains("Recovered!") {
                                    saw_output = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(saw_output, "Should receive output from attempt 3");
    assert_eq!(
        server.request_count.load(Ordering::SeqCst),
        3,
        "Should have auto-recalled exactly 3 times before succeeding"
    );
}

#[tokio::test]
async fn systematic_auto_recall_stops_at_max_retries() {
    // Fail all attempts with 500
    let server = FlakyServer::spawn(10).await;
    let base_url = format!("http://{}", server.addr);
    let params = test_params(base_url);

    let (_cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
    let mut stream = generate_output(params, cancel_rx)
        .await
        .expect("Should create response stream");

    let mut had_error = false;
    while let Some(event_res) = stream.next().await {
        if event_res.is_err() {
            had_error = true;
            break;
        }
    }

    assert!(had_error, "Should fail with error after exhausting 3 attempts");
    assert_eq!(
        server.request_count.load(Ordering::SeqCst),
        4,
        "Should make the initial request plus exactly 3 auto-recalls"
    );
}
