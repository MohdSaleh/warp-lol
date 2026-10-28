//! Opaque wire representation for tool calls the harness executes itself.
//!
//! A call the harness runs in-process still has to live in task history: the
//! next provider turn and any follow-up user request reconstruct the
//! transcript from it, and the loop-control ledger counts it against the run
//! budget. Persisting it as a normal [`api::message::tool_call::Tool`] variant
//! would make the client execute it a second time, so it is stored as the
//! protocol's opaque `Server` call plus a `Server` result. The client renders
//! neither and executes neither; [`super::api_tool_to_openai`] and
//! [`super::render_tool_result`] decode the payload back into the transcript.
//!
//! Payload shape: `{"version":1,"name":"read_files","arguments":{...}}`.

use serde_json::Value;
use warp_multi_agent_api as api;

const PAYLOAD_VERSION: u64 = 1;

pub(crate) fn encode_call(name: &str, arguments: &Value) -> String {
    serde_json::json!({
        "version": PAYLOAD_VERSION,
        "name": name,
        "arguments": arguments,
    })
    .to_string()
}

/// Returns `(provider tool name, JSON-encoded arguments)` for a call the
/// harness persisted, or `None` for any other server tool payload.
pub(crate) fn decode_call(tool: &api::message::tool_call::Tool) -> Option<(String, String)> {
    let api::message::tool_call::Tool::Server(server) = tool else {
        return None;
    };
    let payload: Value = serde_json::from_str(&server.payload).ok()?;
    if payload.get("version")?.as_u64()? != PAYLOAD_VERSION {
        return None;
    }
    let name = payload.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let arguments = payload
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    Some((name.to_string(), arguments.to_string()))
}

pub(crate) fn decode_result(result: &api::message::ToolCallResult) -> Option<String> {
    match result.result.as_ref()? {
        api::message::tool_call_result::Result::Server(server) => {
            Some(server.serialized_result.clone())
        }
        _ => None,
    }
}
