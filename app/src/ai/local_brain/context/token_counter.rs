use crate::ai::local_brain::ChatMessage;

/// Fast local approximation for token counting.
/// Reverts to provider counting when we near the context boundary.
///
/// Heuristic: 1 token per 4 chars on average.
pub fn estimate_tokens_for_messages(messages: &[ChatMessage]) -> u64 {
    let mut total_tokens = 0u64;

    for msg in messages {
        let mut msg_chars = msg.role.len() + 16;
        let is_code_heavy = msg.role == "tool" || msg.role == "assistant";

        if let Some(content) = &msg.content {
            msg_chars += content.len();
        }
        if let Some(tool_call_id) = &msg.tool_call_id {
            msg_chars += tool_call_id.len();
        }
        if let Some(tool_calls) = &msg.tool_calls {
            for call in tool_calls {
                msg_chars += call.id.len();
                msg_chars += call.kind.len();
                msg_chars += call.function.name.len();
                msg_chars += call.function.arguments.len();
            }
        }

        let divisor = if is_code_heavy { 3 } else { 4 };
        total_tokens += (msg_chars / divisor) as u64;
    }

    total_tokens
}

#[cfg(test)]
#[path = "token_counter_tests.rs"]
mod tests;

pub fn estimate_tokens_for_string(text: &str) -> u64 {
    (text.len() / 4) as u64
}
