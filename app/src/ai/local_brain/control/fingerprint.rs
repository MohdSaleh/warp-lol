use prost::Message;
use sha2::{Digest, Sha256};
use warp_multi_agent_api as api;

pub fn fingerprint_tool_call(call: &api::message::ToolCall) -> String {
    let mut hasher = Sha256::new();

    let mut name = String::new();
    let mut args = String::from("{}");

    if let Some(tool_enum) = call.tool.as_ref() {
        if let Some((server_name, server_args)) =
            crate::ai::local_brain::tools::server_tool::decode_call(tool_enum)
        {
            name = server_name;
            args = server_args;
        } else {
            for def in crate::ai::local_brain::tools::registry::BUILTIN_TOOL_REGISTRY {
                if let Some((n, a)) = (def.encode_history)(tool_enum) {
                    name = n;
                    args = a;
                    break;
                }
            }
        }
    }

    hasher.update(name.as_bytes());

    use crate::ai::local_brain::tools::registry::{FingerprintSpec, LocalToolRegistry};
    let spec = LocalToolRegistry::find_by_provider_name(&name)
        .map(|def| def.fingerprint_spec)
        .unwrap_or(FingerprintSpec::ExactMatch);

    match spec {
        FingerprintSpec::ExactMatch => {
            hasher.update(args.as_bytes());
        }
        FingerprintSpec::ExcludeAllEphemeral => {
            // hash name only
        }
        FingerprintSpec::IncludeSpecificFields(fields) => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&args) {
                if let Some(obj) = parsed.as_object() {
                    let mut filtered = serde_json::Map::new();
                    for &field in fields {
                        if let Some(val) = obj.get(field) {
                            filtered.insert(field.to_string(), val.clone());
                        }
                    }
                    hasher.update(
                        serde_json::to_string(&filtered)
                            .unwrap_or_default()
                            .as_bytes(),
                    );
                } else {
                    hasher.update(args.as_bytes());
                }
            } else {
                hasher.update(args.as_bytes());
            }
        }
    }

    format!("{:x}", hasher.finalize())
}

/// Computes a deterministic fingerprint for a tool observation/result.
pub fn fingerprint_observation(result: &api::message::ToolCallResult) -> String {
    let mut hasher = Sha256::new();

    // Hash the enclosing message with an empty call ID so the result payload
    // and context are retained while the retry-specific identifier is ignored.
    let canonical = api::message::ToolCallResult {
        tool_call_id: String::new(),
        context: result.context.clone(),
        result: result.result.clone(),
    };
    hasher.update(canonical.encode_to_vec());

    format!("{:x}", hasher.finalize())
}

pub fn hash_history(tasks: &[api::Task]) -> String {
    let mut hasher = Sha256::new();
    for task in tasks {
        hasher.update(task.encode_to_vec());
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "fingerprint_tests.rs"]
mod tests;
