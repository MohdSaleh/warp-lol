use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use super::*;

struct EchoExecutor;

#[async_trait]
impl HarnessToolExecutor for EchoExecutor {
    fn tool_name(&self) -> &str {
        "echo"
    }

    async fn execute(&self, call: &HarnessToolCall, _context: &ToolContext) -> ToolOutcome {
        ToolOutcome::success(call.arguments.to_string())
    }
}

struct RejectingExecutor;

#[async_trait]
impl HarnessToolExecutor for RejectingExecutor {
    fn tool_name(&self) -> &str {
        "egress"
    }

    async fn execute(&self, _call: &HarnessToolCall, _context: &ToolContext) -> ToolOutcome {
        ToolOutcome::rejected("blocked by local-only policy")
    }
}

struct BarrierExecutor {
    barrier: Arc<tokio::sync::Barrier>,
}

#[async_trait]
impl HarnessToolExecutor for BarrierExecutor {
    fn tool_name(&self) -> &str {
        "sync"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, call: &HarnessToolCall, _context: &ToolContext) -> ToolOutcome {
        self.barrier.wait().await;
        ToolOutcome::success(call.arguments["id"].as_str().unwrap_or_default())
    }
}

fn call(name: &str, arguments: serde_json::Value) -> HarnessToolCall {
    HarnessToolCall {
        call_id: "call-1".to_string(),
        name: name.to_string(),
        arguments,
    }
}

#[tokio::test]
async fn registry_executes_registered_tool() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(EchoExecutor));

    let outcome = registry
        .execute(
            &call("echo", serde_json::json!({ "text": "hi" })),
            &ToolContext::default(),
        )
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Success);
    assert!(outcome.content.contains("\"hi\""));
}

#[tokio::test]
async fn registry_reports_unknown_tool_as_error() {
    let registry = ToolRegistry::new();
    let outcome = registry
        .execute(&call("nope", serde_json::json!({})), &ToolContext::default())
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Error);
    assert_eq!(outcome.content, "unsupported tool: nope");
    assert!(!outcome.is_success());
}

#[tokio::test]
async fn registry_surfaces_rejections() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(RejectingExecutor));

    let outcome = registry
        .execute(&call("egress", serde_json::json!({})), &ToolContext::default())
        .await;

    assert_eq!(outcome.status, ToolOutcomeStatus::Rejected);
    assert!(!outcome.is_success());
}

#[test]
fn registry_tracks_registered_tools() {
    let mut registry = ToolRegistry::new();
    assert!(registry.is_empty());

    registry.register(Box::new(EchoExecutor));

    assert_eq!(registry.len(), 1);
    assert!(registry.contains("echo"));
    assert!(!registry.contains("egress"));
}

#[test]
fn outcome_constructors_set_status() {
    assert_eq!(ToolOutcome::success("ok").status, ToolOutcomeStatus::Success);
    assert_eq!(ToolOutcome::error("boom").status, ToolOutcomeStatus::Error);
    assert_eq!(
        ToolOutcome::rejected("no").status,
        ToolOutcomeStatus::Rejected
    );
}

#[tokio::test]
async fn read_only_batches_run_concurrently_and_keep_call_order() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(BarrierExecutor {
        barrier: Arc::new(tokio::sync::Barrier::new(2)),
    }));
    let calls = vec![
        call("sync", serde_json::json!({ "id": "a" })),
        call("sync", serde_json::json!({ "id": "b" })),
    ];

    let outcomes = tokio::time::timeout(
        Duration::from_secs(2),
        registry.execute_batch(&calls, &ToolContext::default(), 2),
    )
    .await
    .expect("read-only batch did not run concurrently");

    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].content, "a");
    assert_eq!(outcomes[1].content, "b");
}

#[tokio::test]
async fn mutating_batches_run_in_call_order() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(EchoExecutor));
    let calls = vec![
        call("echo", serde_json::json!({ "id": "a" })),
        call("echo", serde_json::json!({ "id": "b" })),
    ];

    let outcomes = registry
        .execute_batch(&calls, &ToolContext::default(), 4)
        .await;

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes[0].content.contains("\"a\""));
    assert!(outcomes[1].content.contains("\"b\""));
}
