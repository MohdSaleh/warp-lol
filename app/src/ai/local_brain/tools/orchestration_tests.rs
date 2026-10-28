use serde_json::json;
use warp_multi_agent_api::message::tool_call as tc;
use warp_multi_agent_api::run_agents;

use super::*;

fn valid_args() -> Value {
    json!({
        "summary": "Investigate a build regression",
        "base_prompt": "Work in /tmp/project and preserve existing changes.",
        "harness": "codex",
        "model_id": "test-model",
        "skills": [{ "kind": "path", "value": "/tmp/project/SKILL.md" }],
        "agent_run_configs": [{
            "name": "compiler-audit",
            "prompt": "Run cargo check and identify the first root cause.",
            "title": "Audit compiler errors"
        }],
        "plan_id": "plan-1"
    })
}

#[test]
fn run_agents_codec_preserves_child_context_and_forces_local_execution() {
    let decoded = decode_run_agents("run_agents", valid_args()).unwrap();
    let tc::Tool::RunAgents(call) = encode_run_agents(decoded).unwrap() else {
        panic!("expected run_agents tool");
    };

    assert_eq!(
        call.base_prompt,
        "Work in /tmp/project and preserve existing changes."
    );
    assert_eq!(call.agent_run_configs.len(), 1);
    assert_eq!(
        call.agent_run_configs[0].prompt,
        "Run cargo check and identify the first root cause."
    );
    assert!(matches!(
        call.execution_mode,
        Some(run_agents::ExecutionMode::Local(_))
    ));
}

#[test]
fn run_agents_history_preserves_the_exact_delegated_task() {
    let tool = encode_run_agents(valid_args()).unwrap();
    let (name, arguments) = history_run_agents(&tool).unwrap();
    let arguments: Value = serde_json::from_str(&arguments).unwrap();

    assert_eq!(name, "run_agents");
    assert_eq!(
        arguments["agent_run_configs"][0]["prompt"],
        "Run cargo check and identify the first root cause."
    );
    assert_eq!(arguments["execution_mode"], "local");
}

#[test]
fn run_agents_rejects_empty_and_duplicate_agent_names() {
    let empty = json!({ "agent_run_configs": [] });
    assert!(decode_run_agents("run_agents", empty).is_err());

    let duplicate = json!({
        "agent_run_configs": [
            { "name": "audit", "prompt": "one" },
            { "name": "audit", "prompt": "two" }
        ]
    });
    assert!(decode_run_agents("run_agents", duplicate).is_err());
}
