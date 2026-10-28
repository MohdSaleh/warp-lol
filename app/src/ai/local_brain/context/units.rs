pub enum ContextUnitPriority {
    High,
    Medium,
    Low,
}

pub struct AtomicContextUnit {
    pub content: String,
    pub priority: ContextUnitPriority,
    pub token_estimate: u64,
}

impl AtomicContextUnit {
    pub fn new(content: String, priority: ContextUnitPriority, token_estimate: u64) -> Self {
        Self {
            content,
            priority,
            token_estimate,
        }
    }
}
