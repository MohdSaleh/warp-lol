use super::token_counter::estimate_tokens_for_messages;
use crate::ai::local_brain::tools::registry::{ContextRetentionPolicy, BUILTIN_TOOL_REGISTRY};
use crate::ai::local_brain::ChatMessage;

fn tool_name_for_result(messages: &[ChatMessage], tool_call_id: &str) -> Option<String> {
    for msg in messages {
        if let Some(tool_calls) = &msg.tool_calls {
            for call in tool_calls {
                if call.id == tool_call_id {
                    return Some(call.function.name.clone());
                }
            }
        }
    }
    None
}

fn retention_policy_for_tool(tool_name: &str) -> Option<ContextRetentionPolicy> {
    BUILTIN_TOOL_REGISTRY
        .iter()
        .find(|def| def.provider_name == tool_name)
        .map(|def| def.context_retention)
}

pub fn compact_context(messages: &mut Vec<ChatMessage>, target_tokens: u64) -> bool {
    let mut compacted = false;

    // Fast path: if under budget, do nothing.
    if estimate_tokens_for_messages(messages) <= target_tokens {
        return false;
    }

    // Pass 1: DropOnCompaction tool results
    for i in 0..messages.len() {
        if estimate_tokens_for_messages(messages) <= target_tokens {
            return compacted;
        }

        let is_drop = {
            let msg = &messages[i];
            if msg.role == "tool" {
                if let Some(ref id) = msg.tool_call_id {
                    if let Some(tool_name) = tool_name_for_result(messages, id) {
                        retention_policy_for_tool(&tool_name)
                            == Some(ContextRetentionPolicy::DropOnCompaction)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_drop {
            messages[i].content = Some("[tool result dropped during compaction]".to_string());
            compacted = true;
        }
    }

    // Pass 2: ReplaceWithSummary tool results
    for i in 0..messages.len() {
        if estimate_tokens_for_messages(messages) <= target_tokens {
            return compacted;
        }

        let is_summarize = {
            let msg = &messages[i];
            if msg.role == "tool" {
                if let Some(ref id) = msg.tool_call_id {
                    if let Some(tool_name) = tool_name_for_result(messages, id) {
                        retention_policy_for_tool(&tool_name)
                            == Some(ContextRetentionPolicy::ReplaceWithSummary)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_summarize {
            if let Some(ref mut content) = messages[i].content {
                if content.len() > 500 {
                    let mut new_content = content.chars().take(500).collect::<String>();
                    new_content.push_str("…[truncated]");
                    *content = new_content;
                    compacted = true;
                }
            }
        }
    }

    // Pass 3: Oldest complete user/assistant exchanges (except system prompt, newest user message, and newest 2 exchanges)
    // We want to remove pairs of (user, assistant).
    // An exchange is defined as a user message followed by the assistant's responses (including tool calls and tool results) until the next user message.
    // However, the rule says "oldest complete user/assistant exchanges except: the system prompt, the newest user message, and the newest 2 exchanges are NEVER removed."
    // Let's identify the start of exchanges.

    // We can find the indices of all 'user' messages.
    let mut user_indices = Vec::new();
    for (i, msg) in messages.iter().enumerate() {
        if msg.role == "user" {
            user_indices.push(i);
        }
    }

    // We must keep the newest user message, and the newest 2 exchanges.
    // The "newest 2 exchanges" means the last 2 user messages and everything after them.
    // So we must keep at least the last 3 user messages (newest user message + 2 exchanges).
    // Wait, the newest user message might be part of the newest exchange?
    // Let's just say we keep the last 3 user messages and everything after them.
    if user_indices.len() > 3 {
        // We can remove exchanges starting from user_indices[0] up to user_indices[user_indices.len() - 3].
        // To remove an exchange, we delete all messages from this user message up to (but not including) the next user message.
        // We should do this from oldest to newest.
        // Because removing elements shifts indices, it's easier to remove one exchange, then re-evaluate.

        loop {
            if estimate_tokens_for_messages(messages) <= target_tokens {
                return compacted;
            }

            // Recalculate user indices
            let mut current_user_indices = Vec::new();
            for (i, msg) in messages.iter().enumerate() {
                if msg.role == "user" {
                    current_user_indices.push(i);
                }
            }

            if current_user_indices.len() <= 3 {
                break;
            }

            // Remove the oldest exchange: from current_user_indices[0] to current_user_indices[1]
            let start = current_user_indices[0];
            let end = current_user_indices[1];

            // Remove the range
            messages.drain(start..end);
            compacted = true;
        }
    }

    // Pass 4: Emergency truncation of any oversized tool results if still over budget
    if estimate_tokens_for_messages(messages) > target_tokens {
        for msg in messages.iter_mut() {
            if msg.role == "tool" {
                if let Some(ref mut content) = msg.content {
                    if content.len() > 1000 {
                        let mut new_content = content.chars().take(1000).collect::<String>();
                        new_content.push_str("…[truncated to fit context budget]");
                        *content = new_content;
                        compacted = true;
                    }
                }
            }
        }
    }

    compacted
}

#[cfg(test)]
#[path = "compaction_tests.rs"]
mod tests;
