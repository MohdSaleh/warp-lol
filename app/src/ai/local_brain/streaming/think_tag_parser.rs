use super::protocol::CanonicalStreamEvent;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum ThinkTagState {
    #[default]
    Normal,
    MatchingStart(String),
    Inside,
    MatchingEnd(String),
}

#[derive(Default, Debug)]
pub struct ThinkTagParser {
    state: ThinkTagState,
}

impl ThinkTagParser {
    pub fn process(&mut self, chunk: &str) -> Vec<CanonicalStreamEvent> {
        let mut events = Vec::new();
        self.process_internal(chunk, &mut events);
        Self::collapse_events(events)
    }

    pub fn flush(&mut self) -> Vec<CanonicalStreamEvent> {
        let mut events = Vec::new();
        match std::mem::take(&mut self.state) {
            ThinkTagState::Normal => {}
            ThinkTagState::MatchingStart(buf) => {
                events.push(CanonicalStreamEvent::OutputTextDelta(buf));
            }
            ThinkTagState::Inside => {
                self.state = ThinkTagState::Inside;
            }
            ThinkTagState::MatchingEnd(buf) => {
                // If we were matching the end but stream ended, it's just reasoning text.
                events.push(CanonicalStreamEvent::ReasoningSummaryDelta(buf));
                self.state = ThinkTagState::Inside;
            }
        }
        events
    }

    fn process_internal(&mut self, chunk: &str, events: &mut Vec<CanonicalStreamEvent>) {
        let mut normal_buf = String::new();
        let mut reasoning_buf = String::new();

        let mut flush_normal = |normal_buf: &mut String, events: &mut Vec<CanonicalStreamEvent>| {
            if !normal_buf.is_empty() {
                events.push(CanonicalStreamEvent::OutputTextDelta(std::mem::take(
                    normal_buf,
                )));
            }
        };

        let mut flush_reasoning =
            |reasoning_buf: &mut String, events: &mut Vec<CanonicalStreamEvent>| {
                if !reasoning_buf.is_empty() {
                    events.push(CanonicalStreamEvent::ReasoningSummaryDelta(std::mem::take(
                        reasoning_buf,
                    )));
                }
            };

        for c in chunk.chars() {
            let state = std::mem::take(&mut self.state);

            match state {
                ThinkTagState::Normal => {
                    if c == '<' {
                        flush_normal(&mut normal_buf, events);
                        self.state = ThinkTagState::MatchingStart(String::from("<"));
                    } else {
                        normal_buf.push(c);
                        self.state = ThinkTagState::Normal;
                    }
                }
                ThinkTagState::MatchingStart(mut buf) => {
                    buf.push(c);
                    if buf == "<think>" {
                        self.state = ThinkTagState::Inside;
                    } else if "<think>".starts_with(buf.as_str()) {
                        self.state = ThinkTagState::MatchingStart(buf);
                    } else {
                        // Failed to match. Emit `<` as normal text.
                        flush_normal(&mut normal_buf, events);
                        events.push(CanonicalStreamEvent::OutputTextDelta(String::from("<")));

                        // Set state to normal and process the rest of the buffer recursively
                        self.state = ThinkTagState::Normal;
                        self.process_internal(&buf[1..], events);
                    }
                }
                ThinkTagState::Inside => {
                    if c == '<' {
                        flush_reasoning(&mut reasoning_buf, events);
                        self.state = ThinkTagState::MatchingEnd(String::from("<"));
                    } else {
                        reasoning_buf.push(c);
                        self.state = ThinkTagState::Inside;
                    }
                }
                ThinkTagState::MatchingEnd(mut buf) => {
                    buf.push(c);
                    if buf == "</think>" {
                        self.state = ThinkTagState::Normal;
                    } else if "</think>".starts_with(buf.as_str()) {
                        self.state = ThinkTagState::MatchingEnd(buf);
                    } else {
                        // Failed to match. Emit `<` as reasoning text.
                        flush_reasoning(&mut reasoning_buf, events);
                        events.push(CanonicalStreamEvent::ReasoningSummaryDelta(String::from(
                            "<",
                        )));

                        self.state = ThinkTagState::Inside;
                        self.process_internal(&buf[1..], events);
                    }
                }
            }
        }

        flush_normal(&mut normal_buf, events);
        flush_reasoning(&mut reasoning_buf, events);
    }

    fn collapse_events(events: Vec<CanonicalStreamEvent>) -> Vec<CanonicalStreamEvent> {
        let mut collapsed: Vec<CanonicalStreamEvent> = Vec::new();
        for event in events {
            match (collapsed.last_mut(), event) {
                (
                    Some(CanonicalStreamEvent::OutputTextDelta(last_str)),
                    CanonicalStreamEvent::OutputTextDelta(new_str),
                ) => {
                    last_str.push_str(&new_str);
                }
                (
                    Some(CanonicalStreamEvent::ReasoningSummaryDelta(last_str)),
                    CanonicalStreamEvent::ReasoningSummaryDelta(new_str),
                ) => {
                    last_str.push_str(&new_str);
                }
                (_, event) => {
                    collapsed.push(event);
                }
            }
        }
        collapsed
    }
}
