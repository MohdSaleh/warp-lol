use crate::server::server_api::ai::AgentResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStatus {
    Pending,
    Running { active_index: usize },
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct PipelineTracker {
    pub agents: Vec<AgentResponse>,
    pub status: PipelineStatus,
}

impl PipelineTracker {
    pub fn new(agents: Vec<AgentResponse>) -> Self {
        Self {
            agents,
            status: PipelineStatus::Pending,
        }
    }

    pub fn start(&mut self) {
        if !self.agents.is_empty() {
            self.status = PipelineStatus::Running { active_index: 0 };
        } else {
            self.status = PipelineStatus::Completed;
        }
    }

    pub fn advance(&mut self) -> bool {
        if let PipelineStatus::Running { active_index } = self.status {
            let next_index = active_index + 1;
            if next_index < self.agents.len() {
                self.status = PipelineStatus::Running {
                    active_index: next_index,
                };
                true
            } else {
                self.status = PipelineStatus::Completed;
                false
            }
        } else {
            false
        }
    }

    pub fn fail(&mut self, error: String) {
        self.status = PipelineStatus::Failed(error);
    }

    pub fn active_agent(&self) -> Option<&AgentResponse> {
        if let PipelineStatus::Running { active_index } = self.status {
            self.agents.get(active_index)
        } else {
            None
        }
    }
}
