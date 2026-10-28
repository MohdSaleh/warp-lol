use warpui::App;

use crate::ai::local_byok::is_local_profile;
use crate::auth::{AuthManager, AuthStateProvider};
use crate::server::server_api::ServerApiProvider;

#[test]
fn local_profile_gating() {
    App::test((), |mut app| async move {
        app.update(|ctx| {
            // Mock required dependencies
            ctx.add_singleton_model(|_| ServerApiProvider::new_for_test());
            ctx.add_singleton_model(|_| AuthStateProvider::new_for_test());
            ctx.add_singleton_model(AuthManager::new_for_test);

            // The fork defaults to local-only; LocalByokAgent remains compatible
            // but is no longer the source of truth for local profile.
            assert!(is_local_profile(ctx));
        });
    });
}
