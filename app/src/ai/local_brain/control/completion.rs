use super::ledger::{LocalAgentRunLedger, LocalAgentTerminationReason};

pub fn validate_final_answer(
    _ledger: &mut LocalAgentRunLedger,
    _answer: &str,
) -> Result<(), LocalAgentTerminationReason> {
    // Check if the final answer actually satisfies the goal if we have a supervisor
    // Or if it's empty, etc.
    Ok(())
}
