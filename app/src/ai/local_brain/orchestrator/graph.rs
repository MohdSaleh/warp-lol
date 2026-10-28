use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeId {
    FileIndexer,
    TerminalParser,
    CodeGenerator,
    Reviewer,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transition {
    Next(NodeId),
    Cycle,
    Terminate(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SharedGraphMemory {
    pub indexed_files: Vec<String>,
    pub terminal_outputs: Vec<String>,
    pub generated_code: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphState {
    pub current_node: NodeId,
    pub step_count: usize,
    pub max_steps: usize,
    pub memory: SharedGraphMemory,
    pub history_nodes: Vec<NodeId>,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            current_node: NodeId::FileIndexer,
            step_count: 0,
            max_steps: 10,
            memory: SharedGraphMemory::default(),
            history_nodes: Vec::new(),
        }
    }
}

pub struct GraphEngine {
    pub state: GraphState,
}

impl GraphEngine {
    pub fn new(initial_node: NodeId, max_steps: usize) -> Self {
        Self {
            state: GraphState {
                current_node: initial_node,
                step_count: 0,
                max_steps,
                memory: SharedGraphMemory::default(),
                history_nodes: Vec::new(),
            },
        }
    }

    pub fn from_state(state: GraphState) -> Self {
        Self { state }
    }

    pub fn transition(&mut self, target: Transition) -> Result<NodeId, String> {
        if self.state.step_count >= self.state.max_steps {
            return Err(
                "Max graph execution steps reached (preventing infinite recursion)".to_string(),
            );
        }

        self.state.step_count += 1;
        self.state
            .history_nodes
            .push(self.state.current_node.clone());

        match target {
            Transition::Next(next_node) => {
                self.state.current_node = next_node.clone();
                Ok(next_node)
            }
            Transition::Cycle => Ok(self.state.current_node.clone()),
            Transition::Terminate(reason) => {
                Err(format!("Graph execution terminated cleanly: {reason}"))
            }
        }
    }

    pub fn get_prompt_extension(&self) -> String {
        format!(
            "\n\n--- GRAPH ORCHESTRATION CONTEXT ---\nCurrent Active Agent Node: {:?}\nCycle Step: {}/{}\nIndexed Files: {:?}\nTerminal Outputs Count: {}\n-----------------------------------",
            self.state.current_node,
            self.state.step_count,
            self.state.max_steps,
            self.state.memory.indexed_files,
            self.state.memory.terminal_outputs.len()
        )
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
