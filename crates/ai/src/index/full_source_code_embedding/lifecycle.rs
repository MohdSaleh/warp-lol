use crate::index::full_source_code_embedding::capability_state::CodebaseCapabilityState;

pub struct IndexLifecycleManager;

impl IndexLifecycleManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for IndexLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexLifecycleManager {
    pub fn ensure_index(&self) -> CodebaseCapabilityState {
        CodebaseCapabilityState::Unavailable
    }
}
