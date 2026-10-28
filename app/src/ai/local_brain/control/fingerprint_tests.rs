use warp_multi_agent_api::message::tool_call::{self as tc, Tool};
use warp_multi_agent_api::message::ToolCall;

use super::*;

#[test]
fn test_fingerprint_tool_call() {
    let run_cmd1 = tc::RunShellCommand {
        command: "ls".into(),
        is_read_only: true,
        uses_pager: false,
        citations: vec![],
        is_risky: false,
        risk_category: 0,
        wait_until_complete_value: None,
    };

    let run_cmd2 = tc::RunShellCommand {
        command: "pwd".into(),
        is_read_only: true,
        uses_pager: false,
        citations: vec![],
        is_risky: false,
        risk_category: 0,
        wait_until_complete_value: None,
    };

    let exact_call = ToolCall {
        tool_call_id: "1".into(),
        tool: Some(Tool::RunShellCommand(run_cmd1.clone())),
    };

    let exact_call_diff = ToolCall {
        tool_call_id: "2".into(),
        tool: Some(Tool::RunShellCommand(run_cmd2.clone())),
    };

    // RUN_SHELL_COMMAND uses ExactMatch fingerprinting
    // Differing commands ("ls" vs "pwd") produce different fingerprints.
    assert_ne!(
        fingerprint_tool_call(&exact_call),
        fingerprint_tool_call(&exact_call_diff)
    );

    // Identical commands produce identical fingerprints.
    let exact_call_same = ToolCall {
        tool_call_id: "3".into(),
        tool: Some(Tool::RunShellCommand(run_cmd1)),
    };
    assert_eq!(
        fingerprint_tool_call(&exact_call),
        fingerprint_tool_call(&exact_call_same)
    );
}
