use warpui::AppContext;

use crate::ai::observability::context::AgentObservationContext;
use crate::{send_telemetry_from_ctx, TelemetryEvent};

pub struct StreamTelemetryRecorder {
    pub request_id: String,
    pub model_config_key: String,
    pub connection_duration_ms: u64,
    pub time_to_first_chunk_ms: u64,
    pub decoder_to_warp_latency_ms: u64,
    pub time_to_first_text_ms: u64,
    pub provider_frame_count: usize,
    pub warp_event_count: usize,
    pub stream_duration_ms: u64,
    pub terminal_outcome: String,
    pub cancellation_to_exit_latency_ms: Option<u64>,
    pub reasoning_summary_bytes: usize,
    pub output_bytes: usize,
    pub tool_call_count: usize,
    pub retry_attempt: usize,
    pub observation_context: Option<AgentObservationContext>,
}

impl StreamTelemetryRecorder {
    pub fn emit(&self, ctx: &mut AppContext) {
        // Emit the legacy flat telemetry event
        send_telemetry_from_ctx!(
            TelemetryEvent::LocalStreamCompleted {
                request_id: self.request_id.clone(),
                model_config_key: self.model_config_key.clone(),
                connection_duration_ms: self.connection_duration_ms as u32,
                time_to_first_chunk_ms: self.time_to_first_chunk_ms as u32,
                decoder_to_warp_latency_ms: self.decoder_to_warp_latency_ms as u32,
                time_to_first_text_ms: self.time_to_first_text_ms as u32,
                provider_frame_count: self.provider_frame_count as u32,
                warp_event_count: self.warp_event_count as u32,
                stream_duration_ms: self.stream_duration_ms as u32,
                terminal_outcome: self.terminal_outcome.clone(),
                cancellation_to_exit_latency_ms: self
                    .cancellation_to_exit_latency_ms
                    .map(|d| d as u32)
                    .unwrap_or(0),
                reasoning_summary_bytes: self.reasoning_summary_bytes as u32,
                output_bytes: self.output_bytes as u32,
                tool_call_count: self.tool_call_count as u32,
                retry_attempt: self.retry_attempt as u32,
            },
            ctx
        );

        // TODO: Emit the structured observation context if present
        if let Some(obs_ctx) = &self.observation_context {
            // Write tree to DB or emit unified event here
        }
    }
}
