//! Entry point + gating for the local BYO agent brain.
//!
//! The actual agentic engine lives in [`crate::ai::local_brain`]; this module
//! keeps the small public surface the rest of the app already depends on
//! (gating predicates + the `generate_output` entry the agent runtime calls)
//! and delegates the work.
//!
//! Routing policy: when [`FeatureFlag::LocalByokAgent`] is enabled, *all* agent
//! requests are handled locally by the BYO brain instead of Warp's backend —
//! this is the switch that makes a fork fully self-hosted. It also transparently
//! unlocks agent use for logged-out users (see [`allows_logged_out_ai`]).

use warpui::{AppContext, SingletonEntity as _};

use crate::ai::agent::api::{ConvertToAPITypeError, RequestParams, ResponseStream};
use crate::auth::AuthStateProvider;

#[cfg(test)]
#[path = "local_byok_tests.rs"]
mod tests;

/// True when the local BYO brain is allowed to serve logged-out / anonymous
/// users (used by settings + workspace gating to relax the usual sign-in
/// requirement for BYO features).
pub(crate) fn allows_logged_out_ai(app: &AppContext) -> bool {
    is_local_profile(app)
        && AuthStateProvider::as_ref(app)
            .get()
            .is_anonymous_or_logged_out()
}

/// Returns true if the application is running in "Local Profile" mode (i.e. auth decoupled, fully anonymous operation).
pub(crate) fn is_local_profile(_app: &AppContext) -> bool {
    warp_core::local_runtime::is_local_only()
}

/// Returns true if the application is running in "Connected" mode (i.e. not local profile).
pub(crate) fn is_connected_mode(app: &AppContext) -> bool {
    !is_local_profile(app)
}

/// Whether this request should be handled by the local brain rather than
/// Warp's backend. With the flag on, every request routes locally (the fork's
/// "always use my own brain" switch).
pub(crate) fn should_route_request_locally(_app: &AppContext) -> bool {
    is_local_profile(_app)
}

/// Produces the response-event stream for a request using the local brain.
pub(crate) async fn generate_output(
    params: RequestParams,
    cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    crate::ai::local_brain::generate_output(params, cancellation_rx).await
}
