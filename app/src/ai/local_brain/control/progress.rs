use std::collections::HashSet;

use super::ledger::{LocalAgentRunLedger, ObservationRecord};
use super::profile::LoopControlProfile;

pub fn evaluate_progress(
    ledger: &mut LocalAgentRunLedger,
    profile: &LoopControlProfile,
    new_observations: &[ObservationRecord],
) -> bool {
    let has_progress = if new_observations.is_empty() {
        true
    } else {
        let recent_hashes: HashSet<&str> = ledger
            .recent_observations
            .iter()
            .rev()
            .take(6)
            .map(|o| o.fingerprint_hash.as_str())
            .collect();

        let all_duplicates = new_observations
            .iter()
            .all(|obs| recent_hashes.contains(obs.fingerprint_hash.as_str()));

        !all_duplicates
    };

    for obs in new_observations {
        ledger.recent_observations.push_back(obs.clone());
        if ledger.recent_observations.len() > 32 {
            ledger.recent_observations.pop_front();
        }
    }

    if !has_progress {
        ledger.consecutive_no_progress_batches += 1;
    } else {
        ledger.consecutive_no_progress_batches = 0;
    }

    if ledger.consecutive_no_progress_batches >= profile.hard_no_progress_batch_limit {
        return false;
    }

    true
}
