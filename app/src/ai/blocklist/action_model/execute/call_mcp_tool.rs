use futures::future::BoxFuture;
use futures::FutureExt;
#[cfg(not(target_family = "wasm"))]
use itertools::Itertools;
use settings::Setting;
#[cfg(not(target_family = "wasm"))]
use warpui::SingletonEntity;
use warpui::{Entity, EntityId, ModelContext, ModelHandle};

#[cfg(not(target_family = "wasm"))]
use super::get_server_output_id;
use super::{ActionExecution, AnyActionExecution, ExecuteActionInput, PreprocessActionInput};
use crate::terminal::model::session::active_session::ActiveSession;
#[cfg(not(target_family = "wasm"))]
use crate::{
    ai::{
        agent::{AIAgentAction, AIAgentActionResultType, CallMCPToolResult},
        blocklist::{action_model::AIAgentActionType, BlocklistAIPermissions},
        mcp::TemplatableMCPServerManager,
    },
    send_telemetry_from_app_ctx, TelemetryEvent,
};

pub struct CallMCPToolExecutor {
    _active_session: ModelHandle<ActiveSession>,
    #[allow(dead_code)]
    terminal_view_id: EntityId,
}

impl CallMCPToolExecutor {
    pub fn new(_active_session: ModelHandle<ActiveSession>, terminal_view_id: EntityId) -> Self {
        Self {
            _active_session,
            terminal_view_id,
        }
    }

    #[cfg_attr(target_family = "wasm", allow(unused_variables), allow(dead_code))]
    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        #[cfg(target_family = "wasm")]
        {
            false
        }

        #[cfg(not(target_family = "wasm"))]
        {
            let ExecuteActionInput {
                action:
                    AIAgentAction {
                        action:
                            AIAgentActionType::CallMCPTool {
                                server_id, name, ..
                            },
                        ..
                    },
                conversation_id,
            } = input
            else {
                return false;
            };

            BlocklistAIPermissions::as_ref(ctx).can_call_mcp_tool(
                server_id.as_ref(),
                name.as_str(),
                &conversation_id,
                Some(self.terminal_view_id),
                ctx,
            )
        }
    }

    #[cfg_attr(target_family = "wasm", allow(unused_variables), allow(dead_code))]
    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> {
        #[cfg(target_family = "wasm")]
        {
            ActionExecution::<()>::InvalidAction
        }

        #[cfg(not(target_family = "wasm"))]
        {
            let server_output_id = get_server_output_id(input.conversation_id, ctx);
            let AIAgentAction {
                action:
                    AIAgentActionType::CallMCPTool {
                        server_id,
                        name,
                        input,
                    },
                ..
            } = input.action
            else {
                return ActionExecution::InvalidAction;
            };

            let name_owned = name.to_owned();
            let name_clone = name_owned.clone();

            let serde_json::Value::Object(mut arguments) = input.clone() else {
                return ActionExecution::Sync(AIAgentActionResultType::CallMCPTool(
                    CallMCPToolResult::Error("MCP server tool input not an object".to_owned()),
                ));
            };

            // Intercept internal local_brain web research tools
            if name.as_str() == "web_search" {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let num_results = arguments
                    .get("num_results")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(10) as usize;

                let tinyfish_api_key = ai::api_keys::ApiKeyManager::as_ref(ctx)
                    .keys()
                    .tinyfish
                    .clone()
                    .unwrap_or_default();

                return ActionExecution::new_async(
                    async move {
                        use crate::ai::local_brain::tools::web_research_config::{
                            fetch_url_content, SearchProvider, TinyFishSearchProvider,
                        };
                        let provider = TinyFishSearchProvider::new(tinyfish_api_key.clone());
                        let mut results = match provider.search(&query, num_results).await {
                            Ok(r) => r,
                            Err(e) => {
                                return Ok(rmcp::model::CallToolResult::success(vec![
                                    rmcp::model::Content::text(format!("Search failed: {}", e)),
                                ]));
                            }
                        };

                        let urls_to_fetch: Vec<String> =
                            results.iter().take(3).map(|r| r.url.clone()).collect();
                        let fetch_futures = urls_to_fetch.into_iter().map(|url| {
                            let key = tinyfish_api_key.clone();
                            async move { fetch_url_content(&url, &key).await }
                        });
                        let fetched_contents = futures::future::join_all(fetch_futures).await;

                        for (result, content) in results.iter_mut().take(3).zip(fetched_contents) {
                            result.content = Some(content);
                        }

                        let json_result = serde_json::to_string(&results).unwrap_or_default();
                        Ok(rmcp::model::CallToolResult::success(vec![
                            rmcp::model::Content::text(json_result),
                        ]))
                    },
                    move |res, ctx| handle_call_tool_result(res, server_output_id, name_clone, ctx),
                );
            }

            if name.as_str() == "web_fetch" {
                let urls: Vec<String> = arguments
                    .get("urls")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let tinyfish_api_key = ai::api_keys::ApiKeyManager::as_ref(ctx)
                    .keys()
                    .tinyfish
                    .clone()
                    .unwrap_or_default();

                return ActionExecution::new_async(
                    async move {
                        use crate::ai::local_brain::tools::web_research_config::fetch_url_content;
                        let mut results = std::collections::BTreeMap::new();
                        for url in urls {
                            results.insert(
                                url.clone(),
                                fetch_url_content(&url, &tinyfish_api_key).await,
                            );
                        }

                        let json_result = serde_json::to_string(&results).unwrap_or_default();
                        Ok(rmcp::model::CallToolResult::success(vec![
                            rmcp::model::Content::text(json_result),
                        ]))
                    },
                    move |res, ctx| handle_call_tool_result(res, server_output_id, name_clone, ctx),
                );
            }

            // Prefer the templatable server over the legacy server if both exist.
            // It is possible for both to exist in some tricky race conditions, but in those cases
            // we shouldn't care about the legacy servers.
            let templatable_mcp_manager = TemplatableMCPServerManager::as_ref(ctx);

            // Coerce whole-number f64 args to i64 for fields declared as `"type": "integer"`
            // in the tool's input schema. MCP tool args round-trip through
            // `google.protobuf.Struct` on the wire, which erases the integer/float distinction
            // by storing everything as f64. Without coercion, the ryu formatter serializes
            // whole-number f64 as "5.0", which strict MCP servers (e.g. GoLand) reject for
            // integer-typed fields.
            if let Some(schema) =
                templatable_mcp_manager.tool_input_schema(*server_id, name.as_str())
            {
                coerce_integer_args(&mut arguments, &schema);
            }

            let templatable_peer = if let Some(installation_id) = server_id {
                templatable_mcp_manager
                    .server_with_installation_id_and_tool_name(*installation_id, name.to_owned())
            } else {
                templatable_mcp_manager.server_with_tool_name(name.to_owned())
            };

            let Some(reconnecting_peer) = templatable_peer else {
                return ActionExecution::Sync(AIAgentActionResultType::CallMCPTool(
                    CallMCPToolResult::Error("MCP server for tool not found".to_owned()),
                ));
            };

            let name_owned_inner = name_owned.clone();
            ActionExecution::new_async(
                async move {
                    reconnecting_peer
                        .call_tool(
                            rmcp::model::CallToolRequestParams::new(name_owned_inner)
                                .with_arguments(arguments),
                        )
                        .await
                },
                move |res, ctx| handle_call_tool_result(res, server_output_id, name_clone, ctx),
            )
        }
    }

    pub(super) fn preprocess_action(
        &mut self,
        _action: PreprocessActionInput,
        _ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        futures::future::ready(()).boxed()
    }
}

impl Entity for CallMCPToolExecutor {
    type Event = ();
}

/// Coerces whole-number floats in `args` to integers for fields declared as
/// [`"type": "integer"`](https://json-schema.org/understanding-json-schema/reference/type)
/// in the tool's JSON Schema `input_schema`.
///
/// MCP tool args round-trip through `google.protobuf.Struct` on the wire, whose
/// `NumberValue` stores everything as `f64`. Without this fix, serde_json emits
/// whole-number floats as `"5.0"`, which strict MCP servers reject for integer fields.
///
/// Walks the schema recursively so integer fields nested inside objects, arrays,
/// or `oneOf`/`anyOf`/`allOf` branches are all coerced. `$ref` is not resolved
/// (the root schema would be required) and is skipped.
pub(crate) fn coerce_integer_args(
    args: &mut serde_json::Map<String, serde_json::Value>,
    input_schema: &serde_json::Map<String, serde_json::Value>,
) {
    // Delegate to the recursive walker by wrapping `args` in a borrowed `Value`.
    // This keeps the root-level traversal consistent with nested levels, so
    // top-level `oneOf`/`anyOf`/`allOf` and `additionalProperties` are honored
    // the same way they are deeper in the schema.
    let mut wrapped = serde_json::Value::Object(std::mem::take(args));
    let schema_value = serde_json::Value::Object(input_schema.clone());
    coerce_value_against_schema(&mut wrapped, &schema_value);
    if let serde_json::Value::Object(restored) = wrapped {
        *args = restored;
    }
}

/// Returns true if the schema's `type` declares `"integer"`, including the
/// nullable form `"type": ["integer", "null"]`.
fn schema_declares_integer(schema: &serde_json::Value) -> bool {
    match schema.get("type") {
        Some(serde_json::Value::String(s)) => s == "integer",
        Some(serde_json::Value::Array(types)) => {
            types.iter().any(|t| t.as_str() == Some("integer"))
        }
        _ => false,
    }
}

/// In-place coerces a whole-number `f64` `Number` to `i64`.
fn coerce_number_to_int(n: &mut serde_json::Number) {
    let Some(f) = n.as_f64() else { return };
    if n.is_i64() || n.is_u64() {
        return;
    }
    if f.fract() != 0.0 {
        return;
    }
    if let Ok(i) = i64::try_from(f as i128) {
        *n = serde_json::Number::from(i);
    }
}

/// Recursively walks `value` against `schema`, coercing whole-number f64s to
/// i64 wherever the schema declares `"type": "integer"`. Safe to call against
/// multiple `oneOf`/`anyOf`/`allOf` branches: coercion is a no-op on values
/// the schema does not match.
fn coerce_value_against_schema(value: &mut serde_json::Value, schema: &serde_json::Value) {
    if schema_declares_integer(schema) {
        if let serde_json::Value::Number(n) = value {
            coerce_number_to_int(n);
        }
    }

    // Visit every combinator key independently — a schema may declare more than
    // one of {oneOf, anyOf, allOf} at the same level, and we need to walk every
    // branch in every present combinator. Coercion is monotonic, so visiting
    // branches whose constraints don't match `value` is a safe no-op.
    for combinator in ["oneOf", "anyOf", "allOf"] {
        if let Some(branches) = schema.get(combinator).and_then(|b| b.as_array()) {
            for branch in branches {
                coerce_value_against_schema(value, branch);
            }
        }
    }

    match value {
        serde_json::Value::Object(map) => {
            let properties = schema.get("properties").and_then(|p| p.as_object());
            let additional = schema.get("additionalProperties");
            // Per-key handling for the outer schema only. Per-branch handling
            // for keys covered by `oneOf`/`anyOf`/`allOf` is reached through
            // the top-level combinator recursion above: each branch is invoked
            // with the full `value`, so the branch's own object handling runs
            // and walks its `properties[k]`. Coercion is monotonic so the two
            // passes stack safely.
            for (k, v) in map.iter_mut() {
                if let Some(prop_schema) = properties.and_then(|p| p.get(k)) {
                    coerce_value_against_schema(v, prop_schema);
                } else if let Some(extra_schema) = additional {
                    if extra_schema.is_object() {
                        coerce_value_against_schema(v, extra_schema);
                    }
                }
            }
        }
        serde_json::Value::Array(items) => {
            if let Some(item_schema) = schema.get("items") {
                match item_schema {
                    // `items` as an object schema: applies to every element.
                    serde_json::Value::Object(_) => {
                        for elem in items.iter_mut() {
                            coerce_value_against_schema(elem, item_schema);
                        }
                    }
                    // `items` as an array (tuple validation): positional schemas.
                    serde_json::Value::Array(schemas) => {
                        for (elem, elem_schema) in items.iter_mut().zip(schemas.iter()) {
                            coerce_value_against_schema(elem, elem_schema);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "call_mcp_tool_tests.rs"]
mod tests;

/// Handles the result of a call_tool request, converting it to an AIAgentActionResultType.
#[cfg(not(target_family = "wasm"))]
fn handle_call_tool_result(
    res: Result<rmcp::model::CallToolResult, rmcp::ServiceError>,
    server_output_id: Option<crate::ai::blocklist::action_model::execute::ServerOutputId>,
    tool_name: String,
    ctx: &warpui::AppContext,
) -> AIAgentActionResultType {
    let action_result = match res {
        Ok(result) => {
            // Even if the call was successful, the response could still be an error so we need to check.
            if matches!(result.is_error, Some(true)) {
                let error_message = result
                    .structured_content
                    .map(|content| content.to_string())
                    .unwrap_or_else(|| {
                        let content_str = result
                            .content
                            .into_iter()
                            .filter_map(|content| {
                                use rmcp::model::RawContent::*;
                                if let Text(raw_text_content) = content.raw {
                                    Some(raw_text_content.text)
                                } else {
                                    log::warn!("Error content found unsupported content type");
                                    None
                                }
                            })
                            .collect_vec()
                            .join("\n");
                        if content_str.is_empty() {
                            "MCP tool call returned an error.".to_string()
                        } else {
                            content_str
                        }
                    });
                if !crate::ai::local_byok::is_local_profile(ctx) {
                    send_telemetry_from_app_ctx!(
                        TelemetryEvent::MCPToolCallAccepted {
                            server_output_id,
                            tool_call: tool_name.clone(),
                            error: Some(
                                crate::server::telemetry::MCPServerTelemetryError::ResponseError(
                                    error_message.clone()
                                )
                            ),
                        },
                        ctx
                    );
                }
                CallMCPToolResult::Error(error_message)
            } else {
                if !crate::ai::local_byok::is_local_profile(ctx) {
                    send_telemetry_from_app_ctx!(
                        TelemetryEvent::MCPToolCallAccepted {
                            server_output_id,
                            tool_call: tool_name,
                            error: None,
                        },
                        ctx
                    );
                }
                CallMCPToolResult::Success { result }
            }
        }
        Err(e) => {
            let error_message = e.to_string();
            log::warn!("Executing MCP tool resulted in error: {e:?}");
            if !crate::ai::local_byok::is_local_profile(ctx) {
                send_telemetry_from_app_ctx!(
                    TelemetryEvent::MCPToolCallAccepted {
                        server_output_id,
                        tool_call: tool_name,
                        error: Some(rmcp::RmcpError::Service(e).into()),
                    },
                    ctx
                );
            }
            CallMCPToolResult::Error(error_message)
        }
    };
    AIAgentActionResultType::CallMCPTool(action_result)
}
