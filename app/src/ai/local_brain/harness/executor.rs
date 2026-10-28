use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::output;

/// One tool invocation requested by the model, already decoded from its
/// JSON-arguments wire form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolOutcomeStatus {
    Success,
    Error,
    /// The tool was available to the model but execution was refused, e.g. by
    /// the local-only egress policy.
    Rejected,
}

/// Result of executing one tool call, in the form fed back to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutcome {
    pub status: ToolOutcomeStatus,
    pub content: String,
}

impl ToolOutcome {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            status: ToolOutcomeStatus::Success,
            content: content.into(),
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            status: ToolOutcomeStatus::Error,
            content: content.into(),
        }
    }

    pub fn rejected(content: impl Into<String>) -> Self {
        Self {
            status: ToolOutcomeStatus::Rejected,
            content: content.into(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == ToolOutcomeStatus::Success
    }
}

/// Environment the harness exposes to executors. Executors are UI-free: they
/// receive plain paths and counters, never warpui handles.
#[derive(Debug, Clone, Default)]
pub struct ToolContext {
    pub working_directory: Option<PathBuf>,
    pub turn: u32,
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait HarnessToolExecutor: Send + Sync {
    /// Wire name the model uses, e.g. `read_files`.
    fn tool_name(&self) -> &str;

    /// Whether calls may run concurrently within one batch. Mutating tools keep
    /// the default so a batch containing one executes in call order.
    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, call: &HarnessToolCall, context: &ToolContext) -> ToolOutcome;
}

/// Name-keyed set of executors owned by one harness run.
#[derive(Default)]
pub struct ToolRegistry {
    executors: HashMap<String, Box<dyn HarnessToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, executor: Box<dyn HarnessToolExecutor>) {
        self.executors.insert(executor.tool_name().to_string(), executor);
    }

    pub fn contains(&self, tool_name: &str) -> bool {
        self.executors.contains_key(tool_name)
    }

    pub fn len(&self) -> usize {
        self.executors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }

    pub async fn execute(&self, call: &HarnessToolCall, context: &ToolContext) -> ToolOutcome {
        match self.executors.get(&call.name) {
            Some(executor) => {
                let outcome = executor.execute(call, context).await;
                ToolOutcome {
                    status: outcome.status,
                    content: output::bound(&call.name, outcome.content),
                }
            }
            None => ToolOutcome::error(format!("unsupported tool: {}", call.name)),
        }
    }

    /// Execute one model turn's calls. A batch of read-only calls runs with
    /// bounded concurrency; any batch containing a mutating or unknown tool
    /// runs sequentially so side effects follow call order. Results always
    /// match `calls` order, so the transcript stays deterministic.
    pub async fn execute_batch(
        &self,
        calls: &[HarnessToolCall],
        context: &ToolContext,
        max_concurrency: usize,
    ) -> Vec<ToolOutcome> {
        let concurrency = max_concurrency.max(1).min(calls.len().max(1));
        if calls.len() < 2 || concurrency == 1 || !self.batch_is_read_only(calls) {
            let mut outcomes = Vec::with_capacity(calls.len());
            for call in calls {
                outcomes.push(self.execute(call, context).await);
            }
            return outcomes;
        }

        use futures::stream::{FuturesOrdered, StreamExt};
        let mut outcomes = Vec::with_capacity(calls.len());
        for chunk in calls.chunks(concurrency) {
            let mut pending = FuturesOrdered::new();
            for call in chunk {
                pending.push_back(self.execute(call, context));
            }
            while let Some(outcome) = pending.next().await {
                outcomes.push(outcome);
            }
        }
        outcomes
    }

    fn batch_is_read_only(&self, calls: &[HarnessToolCall]) -> bool {
        calls.iter().all(|call| {
            self.executors
                .get(&call.name)
                .is_some_and(|executor| executor.is_read_only())
        })
    }
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
