use super::{pipeline_handoff_prompt, OrchestrationPipeline};
use crate::ai::orchestration::pipeline_tracker::PipelineTracker;

#[test]
fn pipeline_handoff_keeps_parent_task_and_rejects_init_bootstrap() {
    let prompt = pipeline_handoff_prompt(
        "Fix the orchestration router and preserve the caller's context.",
        "Rust specialist",
        1,
        2,
    );

    assert!(prompt.contains("Fix the orchestration router"));
    assert!(prompt.contains("agent 1 of 2"));
    assert!(prompt.contains("Do not run /init"));
    assert_ne!(prompt.trim(), "/init");
}

#[test]
fn pipeline_retains_parent_task_for_later_agents() {
    let pipeline = OrchestrationPipeline::new(
        PipelineTracker::new(Vec::new()),
        "Keep every child in the current window".to_string(),
    );

    assert_eq!(
        pipeline.parent_task,
        "Keep every child in the current window"
    );
}
