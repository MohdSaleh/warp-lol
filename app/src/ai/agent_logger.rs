use std::fs::OpenOptions;
use std::io::Write;

use chrono::Utc;

/// Logs agent orchestration steps to a local trace file for debugging and observability.
pub fn log_agent_step(agent_id: Option<&str>, event_type: &str, details: &str) {
    let log_path = "/tmp/warp_agent_trace.log";
    let timestamp = Utc::now().to_rfc3339();
    let agent_str = agent_id.unwrap_or("UNKNOWN_AGENT");

    let log_line = format!(
        "[{}] [AGENT:{}] [{}] {}\n",
        timestamp, agent_str, event_type, details
    );

    // Best-effort append; errors are silently ignored as this is a non-critical debug trace.
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = file.write_all(log_line.as_bytes());
    }
}
