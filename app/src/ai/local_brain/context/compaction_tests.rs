use super::*;
use crate::ai::local_brain::context::token_counter::estimate_tokens_for_messages;
use crate::ai::local_brain::{ChatFunctionCall, ChatMessage, ChatToolCall};

#[test]
fn budget_respected() {
    let mut messages = vec![];
    messages.push(ChatMessage::system("System"));

    // Create a 1000-message history with a mix of messages
    for i in 0..300 {
        messages.push(ChatMessage::user(format!("User {}", i)));
        messages.push(ChatMessage::assistant_text(format!("Assistant {}", i)));

        let tool_id = format!("call_{}", i);
        messages.push(ChatMessage::assistant_tool_call(ChatToolCall {
            id: tool_id.clone(),
            kind: "function".into(),
            function: ChatFunctionCall {
                name: "run_shell_command".into(), // a tool that exists, wait, actually we mock the name or just use a known tool name
                arguments: "{}".into(),
            },
        }));

        messages.push(ChatMessage::tool_result(
            tool_id,
            format!("Result {}", i).repeat(100),
        )); // large result
    }

    // Now test with budgets 4k, 32k, 128k
    let budgets = [4000, 32000, 128000];

    for &budget in &budgets {
        let mut msg_copy = messages.clone();
        compact_context(&mut msg_copy, budget);

        assert!(
            estimate_tokens_for_messages(&msg_copy) <= budget,
            "Budget {} not respected",
            budget
        );
    }
}
