use crate::ai::local_brain::streaming::protocol::CanonicalStreamEvent;
use crate::ai::local_brain::streaming::think_tag_parser::ThinkTagParser;

#[test]
fn test_think_tag_parser_basic() {
    let mut parser = ThinkTagParser::default();

    let mut events = parser.process("Hello <think>thinking process</think> World!");
    events.extend(parser.flush());

    assert_eq!(
        events,
        vec![
            CanonicalStreamEvent::OutputTextDelta("Hello ".to_string()),
            CanonicalStreamEvent::ReasoningSummaryDelta("thinking process".to_string()),
            CanonicalStreamEvent::OutputTextDelta(" World!".to_string()),
        ]
    );
}

#[test]
fn test_think_tag_parser_chunked() {
    let mut parser = ThinkTagParser::default();

    let mut events = Vec::new();
    events.extend(parser.process("Hel"));
    events.extend(parser.process("lo <th"));
    events.extend(parser.process("ink>th"));
    events.extend(parser.process("inking"));
    events.extend(parser.process("</th"));
    events.extend(parser.process("ink> Wo"));
    events.extend(parser.process("rld!"));
    events.extend(parser.flush());

    assert_eq!(
        events,
        vec![
            CanonicalStreamEvent::OutputTextDelta("Hel".to_string()),
            CanonicalStreamEvent::OutputTextDelta("lo ".to_string()),
            CanonicalStreamEvent::ReasoningSummaryDelta("th".to_string()),
            CanonicalStreamEvent::ReasoningSummaryDelta("inking".to_string()),
            CanonicalStreamEvent::OutputTextDelta(" Wo".to_string()),
            CanonicalStreamEvent::OutputTextDelta("rld!".to_string()),
        ]
    );
}

#[test]
fn test_think_tag_parser_false_positive() {
    let mut parser = ThinkTagParser::default();

    let mut events = parser.process("Here is a <thing> or <thin<think>real</think>");
    events.extend(parser.flush());

    assert_eq!(
        events,
        vec![
            CanonicalStreamEvent::OutputTextDelta("Here is a <thing> or <thin".to_string()),
            CanonicalStreamEvent::ReasoningSummaryDelta("real".to_string()),
        ]
    );
}

#[test]
fn test_think_tag_parser_interrupted_false_positive() {
    let mut parser = ThinkTagParser::default();

    let mut events = Vec::new();
    events.extend(parser.process("A <thi"));
    events.extend(parser.process("ng"));
    events.extend(parser.flush());

    assert_eq!(
        events,
        vec![
            CanonicalStreamEvent::OutputTextDelta("A ".to_string()),
            CanonicalStreamEvent::OutputTextDelta("<thing".to_string()),
        ]
    );
}
