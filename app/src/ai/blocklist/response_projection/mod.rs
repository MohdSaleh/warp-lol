#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
pub mod context_notice;
pub mod routing_notice;
pub mod source_attribution;
pub mod status;
pub mod terminal_outcome;
pub mod usage_summary;

use crate::ai::agent::{
    AIAgentOutputMessage, AIAgentOutputMessageType, AIAgentText, AIAgentTextSection,
};
use crate::ai::response_experience::projection::RichResponseProjection;

#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Failed to project response")]
    GeneralError,
}

pub struct RichOutputMetadata {
    pub is_done: bool,
    pub partial: bool,
    pub blocked: bool,
}

pub trait ProjectRichResponse {
    fn project_messages(
        &self,
        projection: &RichResponseProjection,
    ) -> Result<Vec<AIAgentOutputMessage>, ProjectionError>;

    fn project_metadata(&self, projection: &RichResponseProjection) -> RichOutputMetadata;
}

pub struct DefaultResponseProjector;

impl ProjectRichResponse for DefaultResponseProjector {
    fn project_messages(
        &self,
        projection: &RichResponseProjection,
    ) -> Result<Vec<AIAgentOutputMessage>, ProjectionError> {
        let mut messages = Vec::new();

        // 1. Emit routing / context notices based on projection.routing and projection.context
        // (Placeholder for now)

        // 2. Emit content text (Reasoning/Output)
        for (id, content_proj) in &projection.content {
            // For now, we don't have text inside ContentProjection since it's TBD,
            // but we'd normally append the text here.
        }

        // 3. Emit Tool calls / Artifacts
        // (Placeholder)

        Ok(messages)
    }

    fn project_metadata(&self, projection: &RichResponseProjection) -> RichOutputMetadata {
        let mut is_done = false;
        let mut partial = false;
        let mut blocked = false;

        if let Some(terminal) = &projection.terminal {
            is_done = true;
            match terminal {
                crate::ai::response_experience::event::RichTerminalOutcome::Partial { .. } => {
                    partial = true
                }
                crate::ai::response_experience::event::RichTerminalOutcome::Blocked { .. } => {
                    blocked = true
                }
                _ => {}
            }
        }

        RichOutputMetadata {
            is_done,
            partial,
            blocked,
        }
    }
}
