#[cfg(test)]
mod golden_traces_tests;
mod in_process_tools;
pub mod mock_server;
#[cfg(test)]
mod provider_timeout;
mod repeated_tool_loop;
mod systematic_auto_recall;
mod think_tag_parser_tests;
mod wrap_up_turn;
#[cfg(test)]
mod openai_chat_tests {
    use crate::ai::local_brain::streaming::openai_chat::OpenAiChatCompletionsDecoder;
    use crate::ai::local_brain::streaming::protocol::{
        CanonicalStreamEvent, ProviderTerminalReason, RawSseEvent, StreamProtocolDecoder,
    };

    #[test]
    fn test_decoder_reasoning_and_output() {
        let mut decoder = OpenAiChatCompletionsDecoder::default();

        let events = decoder.decode(RawSseEvent::Open).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], CanonicalStreamEvent::StreamOpened));

        let data = r#"{"choices":[{"delta":{"reasoning_content":"hello"}}]}"#;
        let events = decoder
            .decode(RawSseEvent::Message(data.to_string()))
            .unwrap();
        assert_eq!(events.len(), 1);
        if let CanonicalStreamEvent::ReasoningSummaryDelta(d) = &events[0] {
            assert_eq!(d, "hello");
        } else {
            panic!("Expected reasoning delta");
        }

        let data2 = r#"{"choices":[{"delta":{"content":"world"}, "finish_reason": "stop"}]}"#;
        let events2 = decoder
            .decode(RawSseEvent::Message(data2.to_string()))
            .unwrap();
        assert_eq!(events2.len(), 2);
        if let CanonicalStreamEvent::OutputTextDelta(d) = &events2[0] {
            assert_eq!(d, "world");
        } else {
            panic!("Expected output delta");
        }
        if let CanonicalStreamEvent::Terminal(reason) = &events2[1] {
            assert!(matches!(reason, ProviderTerminalReason::Stop));
        } else {
            panic!("Expected terminal event");
        }

        let events3 = decoder
            .decode(RawSseEvent::Message("[DONE]".to_string()))
            .unwrap();
        assert_eq!(events3.len(), 1);
        assert!(matches!(events3[0], CanonicalStreamEvent::DoneMarker));
    }

    #[test]
    fn test_decoder_handles_dual_reasoning_and_reasoning_content_fields() {
        let mut decoder = OpenAiChatCompletionsDecoder::default();

        let data = r#"{"choices":[{"delta":{"reasoning":"hello from reasoning","reasoning_content":"hello from reasoning_content"}}]}"#;
        let events = decoder
            .decode(RawSseEvent::Message(data.to_string()))
            .unwrap();
        assert_eq!(events.len(), 1);
        if let CanonicalStreamEvent::ReasoningSummaryDelta(d) = &events[0] {
            assert_eq!(d, "hello from reasoning");
        } else {
            panic!("Expected reasoning delta");
        }
    }

    #[test]
    fn test_decoder_handles_reasoning_field() {
        let mut decoder = OpenAiChatCompletionsDecoder::default();

        let data = r#"{"choices":[{"delta":{"reasoning":"hello reasoning only"}}]}"#;
        let events = decoder
            .decode(RawSseEvent::Message(data.to_string()))
            .unwrap();
        assert_eq!(events.len(), 1);
        if let CanonicalStreamEvent::ReasoningSummaryDelta(d) = &events[0] {
            assert_eq!(d, "hello reasoning only");
        } else {
            panic!("Expected reasoning delta");
        }
    }
}

#[test]
fn emitter_emits_rich_events_for_successful_stream() {
    use crate::ai::agent::conversation::AIConversationId;
    use crate::ai::local_brain::streaming::config::{StreamEmissionConfig, StreamLimitConfig};
    use crate::ai::local_brain::streaming::emitter::WarpStreamEmitter;
    use crate::ai::local_brain::streaming::protocol::{
        CanonicalStreamEvent, ProviderTerminalReason, StreamUsage,
    };
    use crate::ai::response_experience::RichEventPayload;

    let mut emitter = WarpStreamEmitter::new(
        StreamEmissionConfig::default(),
        StreamLimitConfig::default(),
        "task-1".to_string(),
        "request-1".to_string(),
        AIConversationId::new(),
        "model-1".to_string(),
        8,
        0,
    );

    let mut rich_events = Vec::new();
    rich_events.extend(
        emitter
            .apply_event(CanonicalStreamEvent::StreamOpened)
            .unwrap()
            .1,
    );
    rich_events.extend(
        emitter
            .apply_event(CanonicalStreamEvent::OutputTextDelta("hello".to_string()))
            .unwrap()
            .1,
    );
    rich_events.extend(
        emitter
            .apply_event(CanonicalStreamEvent::Usage(StreamUsage {
                prompt_tokens: 3,
                completion_tokens: 2,
                total_tokens: 5,
            }))
            .unwrap()
            .1,
    );
    rich_events.extend(
        emitter
            .apply_event(CanonicalStreamEvent::Terminal(ProviderTerminalReason::Stop))
            .unwrap()
            .1,
    );
    rich_events.extend(
        emitter
            .apply_event(CanonicalStreamEvent::DoneMarker)
            .unwrap()
            .1,
    );
    rich_events.extend(emitter.finalize().unwrap().1);

    assert!(rich_events
        .iter()
        .any(|event| matches!(event.payload, RichEventPayload::Run(_))));
    assert!(rich_events
        .iter()
        .any(|event| matches!(event.payload, RichEventPayload::Content(_))));
    assert!(rich_events
        .iter()
        .any(|event| matches!(event.payload, RichEventPayload::Usage(_))));
    assert!(rich_events
        .iter()
        .any(|event| matches!(event.payload, RichEventPayload::Terminal(_))));
}

#[test]
fn rich_event_sequences_continue_across_attempts_of_one_request() {
    use crate::ai::agent::conversation::AIConversationId;
    use crate::ai::local_brain::streaming::config::{StreamEmissionConfig, StreamLimitConfig};
    use crate::ai::local_brain::streaming::emitter::WarpStreamEmitter;
    use crate::ai::local_brain::streaming::protocol::CanonicalStreamEvent;

    let mut first_attempt = WarpStreamEmitter::new(
        StreamEmissionConfig::default(),
        StreamLimitConfig::default(),
        "task-1".to_string(),
        "request-1".to_string(),
        AIConversationId::new(),
        "model-1".to_string(),
        4,
        0,
    );
    let (_, first_events) = first_attempt
        .apply_event(CanonicalStreamEvent::ReasoningSummaryDelta("thinking".to_string()))
        .unwrap();
    let (_, more_first_events) = first_attempt
        .apply_event(CanonicalStreamEvent::OutputTextDelta("hello".to_string()))
        .unwrap();
    let max_first_sequence = first_events
        .iter()
        .chain(&more_first_events)
        .map(|event| event.sequence)
        .max()
        .expect("first attempt emitted rich events");

    // A retry reuses the request id; its numbering must not repeat the
    // sequences the first attempt already persisted.
    let mut retry = WarpStreamEmitter::new(
        StreamEmissionConfig::default(),
        StreamLimitConfig::default(),
        "task-1".to_string(),
        "request-1".to_string(),
        AIConversationId::new(),
        "model-1".to_string(),
        4,
        first_attempt.sequence(),
    );
    let (_, retry_events) = retry
        .apply_event(CanonicalStreamEvent::OutputTextDelta("again".to_string()))
        .unwrap();

    assert!(!retry_events.is_empty());
    assert!(retry_events
        .iter()
        .all(|event| event.sequence > max_first_sequence));
}

#[test]
fn emitter_translates_todo_updates_before_the_paired_execution_tool() {
    use crate::ai::agent::conversation::AIConversationId;
    use crate::ai::local_brain::streaming::config::{StreamEmissionConfig, StreamLimitConfig};
    use crate::ai::local_brain::streaming::emitter::WarpStreamEmitter;
    use crate::ai::local_brain::streaming::protocol::{
        CanonicalStreamEvent, ProviderTerminalReason,
    };

    let mut emitter = WarpStreamEmitter::new(
        StreamEmissionConfig::default(),
        StreamLimitConfig::default(),
        "task-1".to_string(),
        "request-1".to_string(),
        AIConversationId::new(),
        "model-1".to_string(),
        4,
        0,
    );

    for event in [
        CanonicalStreamEvent::ToolCallStarted {
            index: 0,
            id: Some("todo-call".to_string()),
            name_fragment: Some("update_todos".to_string()),
        },
        CanonicalStreamEvent::ToolArgumentsDelta {
            index: 0,
            fragment: r#"{"operation":"create","todos":[{"id":"T1","title":"Inspect","description":"Read files"}]}"#.to_string(),
        },
        CanonicalStreamEvent::ToolCallStarted {
            index: 1,
            id: Some("shell-call".to_string()),
            name_fragment: Some("run_command".to_string()),
        },
        CanonicalStreamEvent::ToolArgumentsDelta {
            index: 1,
            fragment: r#"{"command":"pwd"}"#.to_string(),
        },
        CanonicalStreamEvent::Terminal(ProviderTerminalReason::ToolCalls),
        CanonicalStreamEvent::DoneMarker,
    ] {
        emitter.apply_event(event).unwrap();
    }

    let (events, _) = emitter.finalize().unwrap();
    let messages = events
        .iter()
        .filter_map(|event| event.r#type.as_ref())
        .find_map(|event| match event {
            warp_multi_agent_api::response_event::Type::ClientActions(actions) => actions
                .actions
                .iter()
                .filter_map(|action| action.action.as_ref())
                .find_map(|action| match action {
                    warp_multi_agent_api::client_action::Action::AddMessagesToTask(add) => {
                        Some(&add.messages)
                    }
                    _ => None,
                }),
            _ => None,
        })
        .expect("todo and execution messages should be emitted together");

    assert!(matches!(
        messages[0].message,
        Some(warp_multi_agent_api::message::Message::UpdateTodos(_))
    ));
    assert!(matches!(
        messages[1].message,
        Some(warp_multi_agent_api::message::Message::ToolCall(_))
    ));
}

#[test]
fn emitter_gracefully_handles_invalid_tool_json_args() {
    use crate::ai::agent::conversation::AIConversationId;
    use crate::ai::local_brain::streaming::config::{StreamEmissionConfig, StreamLimitConfig};
    use crate::ai::local_brain::streaming::emitter::WarpStreamEmitter;
    use crate::ai::local_brain::streaming::protocol::{
        CanonicalStreamEvent, ProviderTerminalReason,
    };

    let mut emitter = WarpStreamEmitter::new(
        StreamEmissionConfig::default(),
        StreamLimitConfig::default(),
        "task-1".to_string(),
        "request-1".to_string(),
        AIConversationId::new(),
        "model-1".to_string(),
        4,
        0,
    );

    let events = vec![
        CanonicalStreamEvent::ToolCallStarted {
            index: 0,
            id: Some("shell-call".to_string()),
            name_fragment: Some("run_command".to_string()),
        },
        CanonicalStreamEvent::ToolArgumentsDelta {
            index: 0,
            fragment: r#"{"command": invalid_json_here}"#.to_string(),
        },
        CanonicalStreamEvent::Terminal(ProviderTerminalReason::ToolCalls),
        CanonicalStreamEvent::DoneMarker,
    ];

    for event in events {
        let result = emitter.apply_event(event);
        assert!(
            result.is_ok(),
            "apply_event should not fail on invalid tool JSON"
        );
    }

    let finalize_result = emitter.finalize();
    assert!(
        finalize_result.is_ok(),
        "finalize should succeed with fallback tool args"
    );
}
