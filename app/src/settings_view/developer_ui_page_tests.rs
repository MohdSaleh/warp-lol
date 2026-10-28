use ::settings::{PrivatePreferences, PublicPreferences, Setting, SettingsManager};
use warpui::{App, SingletonEntity, TypedActionView};
use warpui_extras::user_preferences;

use super::{DeveloperUiSettingsPage, DeveloperUiSettingsPageAction};
use crate::settings::DeveloperUiSettings;
use crate::settings_view::Appearance;

#[test]
fn test_developer_ui_settings_page_actions() {
    App::test((), |mut app| async move {
        // Initialize minimal settings system
        app.update(|ctx| {
            ctx.add_singleton_model(move |_| {
                PublicPreferences::new(
                    Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
                )
            });
            ctx.add_singleton_model(move |_| -> PrivatePreferences {
                PrivatePreferences::new(
                    Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
                )
            });
            ctx.add_singleton_model(|_| SettingsManager::default());
            ctx.add_singleton_model(|_| Appearance::mock());

            DeveloperUiSettings::register(ctx);
        });

        let (_window_id, page_handle) = app
            .add_window(warpui::platform::WindowStyle::NotStealFocus, |_ctx| {
                DeveloperUiSettingsPage::default()
            });

        // Test toggle agent approval
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_agent_approval.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleAgentApproval, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_agent_approval.value());
        });

        // Test toggle agent todos
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_agent_todos.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleAgentTodos, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_agent_todos.value());
        });

        // Test toggle agent config
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_agent_config.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleAgentConfig, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_agent_config.value());
        });

        // Test toggle agent question
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_agent_question.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleAgentQuestion, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_agent_question.value());
        });

        // Test toggle agent diff
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_agent_diff.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleAgentDiff, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_agent_diff.value());
        });

        // Test toggle terminal zero state
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_terminal_zero_state.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleTerminalZeroState, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_terminal_zero_state.value());
        });

        // Test toggle agent zero state
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_agent_zero_state.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleAgentZeroState, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_agent_zero_state.value());
        });

        // Test toggle ssh server choice
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_ssh_server_choice.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleSshServerChoice, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_ssh_server_choice.value());
        });

        // Test toggle prompt suggestions
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_prompt_suggestions.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::TogglePromptSuggestions, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_prompt_suggestions.value());
        });

        // Test toggle incompatible configuration banner
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui
                .force_show_incompatible_configuration_banner
                .value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(
                    &DeveloperUiSettingsPageAction::ToggleIncompatibleConfigurationBanner,
                    ctx,
                );
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui
                .force_show_incompatible_configuration_banner
                .value());
        });

        // Test toggle emacs bindings banner
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_emacs_bindings_banner.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(
                    &DeveloperUiSettingsPageAction::ToggleEmacsBindingsBanner,
                    ctx,
                );
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_emacs_bindings_banner.value());
        });

        // Test toggle control master error banner
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_control_master_error_banner.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(
                    &DeveloperUiSettingsPageAction::ToggleControlMasterErrorBanner,
                    ctx,
                );
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_control_master_error_banner.value());
        });

        // Test toggle osc52 clipboard blocked banner
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui
                .force_show_osc52_clipboard_blocked_banner
                .value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(
                    &DeveloperUiSettingsPageAction::ToggleOsc52ClipboardBlockedBanner,
                    ctx,
                );
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui
                .force_show_osc52_clipboard_blocked_banner
                .value());
        });

        // Test toggle ssh file upload
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_ssh_file_upload.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(&DeveloperUiSettingsPageAction::ToggleSshFileUpload, ctx);
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_ssh_file_upload.value());
        });

        // Test toggle first time cloud agent setup view
        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(!*developer_ui.force_show_first_time_cloud_agent_setup.value());
        });

        app.update(|ctx| {
            page_handle.update(ctx, |page, ctx| {
                page.handle_action(
                    &DeveloperUiSettingsPageAction::ToggleFirstTimeCloudAgentSetup,
                    ctx,
                );
            });
        });

        app.update(|ctx| {
            let developer_ui = DeveloperUiSettings::as_ref(ctx);
            assert!(*developer_ui.force_show_first_time_cloud_agent_setup.value());
        });
    });
}
