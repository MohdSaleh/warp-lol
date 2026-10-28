use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::response::sse::{Event, Sse};
use axum::routing::post;
use axum::Router;
use futures::stream::StreamExt;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
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
                    let (tx, rx) =
                        tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(10);

                    // Accept connection but send nothing for a long time to trigger timeout
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(10)).await;
                        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                    });

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
async fn provider_timeout() {
    // Override the timeout via env var to make the test fast (2000ms = 2s)
    std::env::set_var("WARP_DEV_LOCAL_STREAM_FIRST_EVENT_TIMEOUT_MS", "2000");
    std::env::set_var("WARP_DEV_LOCAL_STREAM_IDLE_TIMEOUT_MS", "2000");

    let server = MockServer::spawn().await;
    let base_url = format!("http://{}", server.addr);

    let params = params_with_model(base_url);

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

    let mut params = params.clone();
    params.tasks = vec![task.clone()];

    let (cancel_tx, cancel_rx) = futures::channel::oneshot::channel();

    let stream_result = generate_output(params, cancel_rx).await;
    let mut stream = stream_result.expect("Should return a stream");

    let mut error_seen = false;
    while let Some(res) = stream.next().await {
        match res {
            Ok(_) => {}
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("first event timeout") || err_str.contains("idle timeout") {
                    error_seen = true;
                }
            }
        }
    }

    assert!(error_seen, "The timeout error should be emitted");
}
