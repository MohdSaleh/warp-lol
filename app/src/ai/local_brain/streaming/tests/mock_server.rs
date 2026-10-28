use std::net::SocketAddr;

use axum::routing::post;
use axum::Router;
use tokio::net::TcpListener;

pub struct MockServer {
    pub addr: SocketAddr,
}

impl MockServer {
    pub async fn spawn() -> Self {
        let app = Router::new().route("/chat/completions", post(mock_completions));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self { addr }
    }
}

async fn mock_completions() -> &'static str {
    ""
}
