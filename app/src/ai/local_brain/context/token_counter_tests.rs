use super::*;
use crate::ai::local_brain::{ChatFunctionCall, ChatMessage, ChatToolCall};

#[test]
fn test_estimate_tokens_for_messages() {
    let msg1 = ChatMessage::user("Hello".into());
    let msg2 = ChatMessage::assistant_text("World".into());
    let msg3 = ChatMessage::assistant_tool_call(ChatToolCall {
        id: "call_1".into(),
        kind: "function".into(),
        function: ChatFunctionCall {
            name: "test_tool".into(),
            arguments: "{}".into(),
        },
    });

    let messages = vec![msg1, msg2, msg3];

    // Calculate expected characters and token heuristic:
    // msg1: user, role=user(4) + content=Hello(5) + 16 = 25 -> 25 / 4 = 6
    // msg2: assistant (code-heavy), role=assistant(9) + content=World(5) + 16 = 30 -> 30 / 3 = 10
    // msg3: assistant (code-heavy), role=assistant(9) + tool_call.id(6) + tool_call.kind(8) + tool_call.function.name(9) + tool_call.function.arguments(2) + 16 = 50 -> 50 / 3 = 16
    // Total tokens = 6 + 10 + 16 = 32

    assert_eq!(estimate_tokens_for_messages(&messages), 32);
}

#[test]
fn test_estimate_tokens_for_string() {
    assert_eq!(estimate_tokens_for_string("1234"), 1);
    assert_eq!(estimate_tokens_for_string("12345"), 1);
    assert_eq!(estimate_tokens_for_string("12345678"), 2);
}
