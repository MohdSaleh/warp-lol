use ai::agent::action_result::{AskUserQuestionAnswerItem, AskUserQuestionResult};
use serde_json::{json, Value};
use warp_multi_agent_api::message::tool_call as tc;

use super::availability::ToolAvailability;
use super::context::ToolRuntimeContext;
use super::errors::{ToolArgumentError, ToolEncodingError};
use super::registry::{
    ContextRetentionPolicy, FingerprintSpec, LocalToolDefinition, LocalToolId,
    ProviderNeutralToolCall, ToolExecutionPolicy, ToolLoopSemantics,
};
use super::results::{ToolResultEnvelope, ToolResultRenderResult};

pub const ASK_USER_QUESTION: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::AskUserQuestion,
    provider_name: "ask_user_question",
    execution_policy: ToolExecutionPolicy::Interactive,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: ask_user_question_available,
    schema: ask_user_question_schema,
    decode: decode_ask_user_question,
    encode_warp_tool: encode_ask_user_question,
    encode_history: history_ask_user_question,
    render_result: render_ask_user_question_result,
};

pub const SUGGEST_PROMPT: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::SuggestPrompt,
    provider_name: "suggest_prompt",
    execution_policy: ToolExecutionPolicy::Interactive,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: suggest_prompt_available,
    schema: suggest_prompt_schema,
    decode: decode_suggest_prompt,
    encode_warp_tool: encode_suggest_prompt,
    encode_history: history_suggest_prompt,
    render_result: render_suggest_prompt_result,
};

pub const SUGGEST_NEW_CONVERSATION: LocalToolDefinition = LocalToolDefinition {
    id: LocalToolId::SuggestNewConversation,
    provider_name: "suggest_new_conversation",
    execution_policy: ToolExecutionPolicy::Interactive,
    loop_semantics: ToolLoopSemantics::StateDependent,
    context_retention: ContextRetentionPolicy::HighValue,
    fingerprint_spec: FingerprintSpec::ExactMatch,
    availability: suggest_new_conversation_available,
    schema: suggest_new_conversation_schema,
    decode: decode_suggest_new_conversation,
    encode_warp_tool: encode_suggest_new_conversation,
    encode_history: history_suggest_new_conversation,
    render_result: render_suggest_new_conversation_result,
};

fn ask_user_question_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn ask_user_question_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "ask_user_question",
            "description": "Ask the user one or more multiple-choice questions.",
            "parameters": {
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": {"type": "string"},
                                "options": {
                                    "type": "array",
                                    "items": {"type": "string"}
                                },
                                "is_multiselect": {"type": "boolean"},
                                "supports_other": {"type": "boolean"}
                            },
                            "required": ["question", "options"]
                        }
                    }
                },
                "required": ["questions"]
            }
        }
    })
}
fn decode_ask_user_question(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_ask_user_question(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    #[derive(serde::Deserialize)]
    struct ArgsQuestion {
        question: String,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default)]
        is_multiselect: bool,
        #[serde(default)]
        supports_other: bool,
    }

    #[derive(serde::Deserialize)]
    struct Args {
        #[serde(default)]
        questions: Vec<ArgsQuestion>,
    }

    let parsed: Args = serde_json::from_value(args)
        .map_err(|e| ToolEncodingError::ConversionFailed(e.to_string()))?;

    let questions = parsed
        .questions
        .into_iter()
        .enumerate()
        .map(|(i, q)| {
            let options = q
                .options
                .into_iter()
                .map(|label| warp_multi_agent_api::ask_user_question::Option { label })
                .collect();
            warp_multi_agent_api::ask_user_question::Question {
                question_id: format!("q{}", i),
                question: q.question,
                question_type: Some(
                    warp_multi_agent_api::ask_user_question::question::QuestionType::MultipleChoice(
                        warp_multi_agent_api::ask_user_question::MultipleChoice {
                            options,
                            recommended_option_index: -1,
                            is_multiselect: q.is_multiselect,
                            supports_other: q.supports_other,
                        },
                    ),
                ),
            }
        })
        .collect();

    Ok(tc::Tool::AskUserQuestion(
        warp_multi_agent_api::AskUserQuestion { questions },
    ))
}
fn history_ask_user_question(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::AskUserQuestion(call) = tool {
        let questions: Vec<_> = call
            .questions
            .iter()
            .map(|question| {
                use warp_multi_agent_api::ask_user_question::question::QuestionType;

                match question.question_type.as_ref() {
                    Some(QuestionType::MultipleChoice(config)) => json!({
                        "question_id": question.question_id,
                        "question": question.question,
                        "options": config.options.iter().map(|option| &option.label).collect::<Vec<_>>(),
                        "recommended_option_index": config.recommended_option_index,
                        "is_multiselect": config.is_multiselect,
                        "supports_other": config.supports_other,
                    }),
                    None => json!({
                        "question_id": question.question_id,
                        "question": question.question,
                    }),
                }
            })
            .collect();
        Some((
            "ask_user_question".into(),
            json!({ "questions": questions }).to_string(),
        ))
    } else {
        None
    }
}

fn answer_data_from_proto(
    success: &warp_multi_agent_api::ask_user_question_result::Success,
) -> Value {
    use warp_multi_agent_api::ask_user_question_result::answer_item::Answer;

    let answers: Vec<_> = success
        .answers
        .iter()
        .map(|answer| match &answer.answer {
            Some(Answer::MultipleChoice(multiple_choice)) => json!({
                "question_id": answer.question_id,
                "selected_options": multiple_choice.selected_options,
                "other_text": multiple_choice.other_text,
                "skipped": false,
            }),
            Some(Answer::Skipped(())) | None => json!({
                "question_id": answer.question_id,
                "selected_options": [],
                "other_text": "",
                "skipped": true,
            }),
        })
        .collect();

    json!({ "answers": answers })
}

fn answer_data_from_internal(answers: &[AskUserQuestionAnswerItem]) -> Value {
    let answers: Vec<_> = answers
        .iter()
        .map(|answer| match answer {
            AskUserQuestionAnswerItem::Answered {
                question_id,
                selected_options,
                other_text,
            } => json!({
                "question_id": question_id,
                "selected_options": selected_options,
                "other_text": other_text,
                "skipped": false,
            }),
            AskUserQuestionAnswerItem::Skipped { question_id } => json!({
                "question_id": question_id,
                "selected_options": [],
                "other_text": "",
                "skipped": true,
            }),
        })
        .collect();

    json!({ "answers": answers })
}

pub(in crate::ai::local_brain) fn render_internal_ask_user_question_result(
    result: &AskUserQuestionResult,
) -> ToolResultRenderResult {
    match result {
        AskUserQuestionResult::Success { answers } => ToolResultEnvelope::success(
            LocalToolId::AskUserQuestion,
            answer_data_from_internal(answers),
        ),
        AskUserQuestionResult::SkippedByAutoApprove { question_ids } => {
            let answers: Vec<_> = question_ids
                .iter()
                .map(|question_id| AskUserQuestionAnswerItem::Skipped {
                    question_id: question_id.clone(),
                })
                .collect();
            ToolResultEnvelope::success(
                LocalToolId::AskUserQuestion,
                answer_data_from_internal(&answers),
            )
        }
        AskUserQuestionResult::Error(message) => Ok(ToolResultEnvelope::error(
            LocalToolId::AskUserQuestion,
            message.clone().into(),
        )),
        AskUserQuestionResult::Cancelled => {
            Ok(ToolResultEnvelope::cancelled(LocalToolId::AskUserQuestion))
        }
    }
}

fn render_ask_user_question_result(
    tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    use warp_multi_agent_api::ask_user_question_result::Result as Sr;
    use warp_multi_agent_api::message::tool_call_result::Result as R;
    match &tcr.result {
        Some(R::AskUserQuestion(r)) => match &r.result {
            Some(Sr::Success(success)) => ToolResultEnvelope::success(
                LocalToolId::AskUserQuestion,
                answer_data_from_proto(success),
            ),
            Some(Sr::Error(e)) => Ok(ToolResultEnvelope::error(
                LocalToolId::AskUserQuestion,
                e.message.clone().into(),
            )),
            None => Ok(ToolResultEnvelope::error(
                LocalToolId::AskUserQuestion,
                "No result".into(),
            )),
        },
        _ => Ok(ToolResultEnvelope::error(
            LocalToolId::AskUserQuestion,
            "Unexpected result type".into(),
        )),
    }
}

fn suggest_prompt_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn suggest_prompt_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "suggest_prompt",
            "description": "Suggest a prompt to the user.",
            "parameters": {
                "type": "object",
                "properties": {
                    "prompt": { "type": "string" },
                    "label": { "type": "string" }
                }
            }
        }
    })
}
fn decode_suggest_prompt(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_suggest_prompt(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    use warp_multi_agent_api::message::tool_call::suggest_prompt::{DisplayMode, PromptChip};
    let prompt = args
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let label = args
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(tc::Tool::SuggestPrompt(tc::SuggestPrompt {
        is_trigger_irrelevant: false,
        display_mode: Some(DisplayMode::PromptChip(PromptChip { prompt, label })),
    }))
}
fn history_suggest_prompt(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    use warp_multi_agent_api::message::tool_call::suggest_prompt::DisplayMode;
    if let tc::Tool::SuggestPrompt(c) = tool {
        let (prompt, label) = match &c.display_mode {
            Some(DisplayMode::PromptChip(chip)) => (chip.prompt.clone(), chip.label.clone()),
            _ => ("".to_string(), "".to_string()),
        };
        Some((
            "suggest_prompt".into(),
            json!({ "prompt": prompt, "label": label }).to_string(),
        ))
    } else {
        None
    }
}
fn render_suggest_prompt_result(
    _tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    ToolResultEnvelope::success(LocalToolId::SuggestPrompt, json!({}))
}

fn suggest_new_conversation_available(_context: &ToolRuntimeContext<'_>) -> ToolAvailability {
    ToolAvailability::Available
}
fn suggest_new_conversation_schema(_context: &ToolRuntimeContext<'_>) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "suggest_new_conversation",
            "description": "Suggest starting a new conversation.",
            "parameters": {
                "type": "object",
                "properties": {
                    "message_id": { "type": "string" }
                }
            }
        }
    })
}
fn decode_suggest_new_conversation(_name: &str, args: Value) -> Result<Value, ToolArgumentError> {
    Ok(args)
}
fn encode_suggest_new_conversation(args: Value) -> Result<tc::Tool, ToolEncodingError> {
    Ok(tc::Tool::SuggestNewConversation(
        tc::SuggestNewConversation {
            message_id: args
                .get("message_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        },
    ))
}
fn history_suggest_new_conversation(tool: &tc::Tool) -> Option<ProviderNeutralToolCall> {
    if let tc::Tool::SuggestNewConversation(c) = tool {
        Some((
            "suggest_new_conversation".into(),
            json!({"message_id": c.message_id}).to_string(),
        ))
    } else {
        None
    }
}
fn render_suggest_new_conversation_result(
    _tcr: &warp_multi_agent_api::message::ToolCallResult,
) -> ToolResultRenderResult {
    ToolResultEnvelope::success(LocalToolId::SuggestNewConversation, json!({}))
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
