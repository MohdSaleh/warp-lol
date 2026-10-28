use std::sync::Arc;

use ::settings::Setting;
use warpui::elements::{
    Align, ChildView, ConstrainedBox, Container, Element, Empty, Expanded, Flex, Hoverable,
    MouseStateHandle, Padding, ParentElement, Shrinkable, Stack, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::ui_components::components::UiComponent;
use warpui::{
    AppContext, Entity, EventContext, ModelContext, SingletonEntity, TypedActionView, View,
    ViewContext, ViewHandle,
};

use crate::settings::DeveloperUiSettings;
use crate::settings_view::settings_page::{MatchData, SettingsPageEvent, SettingsPageMeta};
use crate::settings_view::{Appearance, SettingsSection};

#[derive(Clone, Default)]
pub struct DeveloperUiSettingsPage {
    reconnecting_banner_mouse: MouseStateHandle,
    slow_bootstrap_mouse: MouseStateHandle,
    ssh_drop_mouse: MouseStateHandle,
    jump_to_bottom_mouse: MouseStateHandle,
    inactivity_modal_mouse: MouseStateHandle,
    agent_approval_mouse: MouseStateHandle,
    agent_todos_mouse: MouseStateHandle,
    agent_config_mouse: MouseStateHandle,
    agent_question_mouse: MouseStateHandle,
    agent_diff_mouse: MouseStateHandle,
    terminal_zero_state_mouse: MouseStateHandle,
    agent_zero_state_mouse: MouseStateHandle,
    ssh_server_choice_mouse: MouseStateHandle,
    prompt_suggestions_mouse: MouseStateHandle,
    incompatible_configuration_banner_mouse: MouseStateHandle,
    emacs_bindings_banner_mouse: MouseStateHandle,
    control_master_error_banner_mouse: MouseStateHandle,
    osc52_clipboard_blocked_banner_mouse: MouseStateHandle,
    ssh_file_upload_mouse: MouseStateHandle,
    first_time_cloud_agent_setup_mouse: MouseStateHandle,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeveloperUiSettingsPageAction {
    ToggleReconnectingBanner,
    ToggleSlowBootstrapBanner,
    ToggleSshDropOverlay,
    ToggleJumpToBottom,
    ToggleInactivityModal,
    ToggleAgentApproval,
    ToggleAgentTodos,
    ToggleAgentConfig,
    ToggleAgentQuestion,
    ToggleAgentDiff,
    ToggleTerminalZeroState,
    ToggleAgentZeroState,
    ToggleSshServerChoice,
    TogglePromptSuggestions,
    ToggleIncompatibleConfigurationBanner,
    ToggleEmacsBindingsBanner,
    ToggleControlMasterErrorBanner,
    ToggleOsc52ClipboardBlockedBanner,
    ToggleSshFileUpload,
    ToggleFirstTimeCloudAgentSetup,
}

impl Entity for DeveloperUiSettingsPage {
    type Event = SettingsPageEvent;
}

impl TypedActionView for DeveloperUiSettingsPage {
    type Action = DeveloperUiSettingsPageAction;
    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        let handle = DeveloperUiSettings::handle(ctx);
        match action {
            DeveloperUiSettingsPageAction::ToggleReconnectingBanner => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_reconnecting_banner.value();
                    let _ = settings
                        .force_show_reconnecting_banner
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleSlowBootstrapBanner => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_slow_bootstrap_banner.value();
                    let _ = settings
                        .force_show_slow_bootstrap_banner
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleSshDropOverlay => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_ssh_drop_overlay.value();
                    let _ = settings
                        .force_show_ssh_drop_overlay
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleJumpToBottom => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_jump_to_bottom.value();
                    let _ = settings.force_show_jump_to_bottom.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleInactivityModal => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_inactivity_modal.value();
                    let _ = settings
                        .force_show_inactivity_modal
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleAgentApproval => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_agent_approval.value();
                    let _ = settings.force_show_agent_approval.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleAgentTodos => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_agent_todos.value();
                    let _ = settings.force_show_agent_todos.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleAgentConfig => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_agent_config.value();
                    let _ = settings.force_show_agent_config.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleAgentQuestion => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_agent_question.value();
                    let _ = settings.force_show_agent_question.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleAgentDiff => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_agent_diff.value();
                    let _ = settings.force_show_agent_diff.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleTerminalZeroState => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_terminal_zero_state.value();
                    let _ = settings
                        .force_show_terminal_zero_state
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleAgentZeroState => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_agent_zero_state.value();
                    let _ = settings
                        .force_show_agent_zero_state
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleSshServerChoice => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_ssh_server_choice.value();
                    let _ = settings
                        .force_show_ssh_server_choice
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::TogglePromptSuggestions => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_prompt_suggestions.value();
                    let _ = settings
                        .force_show_prompt_suggestions
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleIncompatibleConfigurationBanner => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings
                        .force_show_incompatible_configuration_banner
                        .value();
                    let _ = settings
                        .force_show_incompatible_configuration_banner
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleEmacsBindingsBanner => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_emacs_bindings_banner.value();
                    let _ = settings
                        .force_show_emacs_bindings_banner
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleControlMasterErrorBanner => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_control_master_error_banner.value();
                    let _ = settings
                        .force_show_control_master_error_banner
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleOsc52ClipboardBlockedBanner => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_osc52_clipboard_blocked_banner.value();
                    let _ = settings
                        .force_show_osc52_clipboard_blocked_banner
                        .set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleSshFileUpload => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_ssh_file_upload.value();
                    let _ = settings.force_show_ssh_file_upload.set_value(!current, ctx);
                });
            }
            DeveloperUiSettingsPageAction::ToggleFirstTimeCloudAgentSetup => {
                handle.update(ctx, |settings, ctx| {
                    let current = *settings.force_show_first_time_cloud_agent_setup.value();
                    let _ = settings
                        .force_show_first_time_cloud_agent_setup
                        .set_value(!current, ctx);
                });
            }
        }
    }
}

impl SettingsPageMeta for DeveloperUiSettingsPage {
    fn section() -> SettingsSection {
        SettingsSection::DeveloperUI
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, _query: &str, _ctx: &mut ViewContext<Self>) -> MatchData {
        MatchData::Uncounted(true)
    }

    fn scroll_to_widget(&mut self, _widget_id: &'static str) {}

    fn clear_highlighted_widget(&mut self) {}
}

fn build_checkbox(
    app: &AppContext,
    label: &str,
    is_checked: bool,
    mouse_state: MouseStateHandle,
    action: DeveloperUiSettingsPageAction,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let mut row = Flex::row().with_child(
        Text::new_inline(label.to_string(), appearance.ui_font_family(), 12.)
            .with_color(appearance.theme().active_ui_text_color().into())
            .finish(),
    );
    let toggle = appearance
        .ui_builder()
        .checkbox(mouse_state, None)
        .check(is_checked)
        .build()
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish();

    row.add_child(
        ConstrainedBox::new(Empty::new().finish())
            .with_width(10.)
            .finish(),
    );
    row.add_child(toggle);
    Container::new(row.finish())
        .with_padding_bottom(10.)
        .finish()
}

impl View for DeveloperUiSettingsPage {
    fn ui_name() -> &'static str {
        "DeveloperUiSettingsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let developer_ui = DeveloperUiSettings::as_ref(app);

        let force_show_reconnecting_banner = *developer_ui.force_show_reconnecting_banner.value();
        let force_show_slow_bootstrap_banner =
            *developer_ui.force_show_slow_bootstrap_banner.value();
        let force_show_ssh_drop_overlay = *developer_ui.force_show_ssh_drop_overlay.value();
        let force_show_jump_to_bottom = *developer_ui.force_show_jump_to_bottom.value();
        let force_show_inactivity_modal = *developer_ui.force_show_inactivity_modal.value();
        let force_show_agent_approval = *developer_ui.force_show_agent_approval.value();
        let force_show_agent_todos = *developer_ui.force_show_agent_todos.value();
        let force_show_agent_config = *developer_ui.force_show_agent_config.value();
        let force_show_agent_question = *developer_ui.force_show_agent_question.value();
        let force_show_agent_diff = *developer_ui.force_show_agent_diff.value();
        let force_show_terminal_zero_state = *developer_ui.force_show_terminal_zero_state.value();
        let force_show_agent_zero_state = *developer_ui.force_show_agent_zero_state.value();
        let force_show_ssh_server_choice = *developer_ui.force_show_ssh_server_choice.value();
        let force_show_prompt_suggestions = *developer_ui.force_show_prompt_suggestions.value();
        let force_show_incompatible_configuration_banner = *developer_ui
            .force_show_incompatible_configuration_banner
            .value();
        let force_show_emacs_bindings_banner =
            *developer_ui.force_show_emacs_bindings_banner.value();
        let force_show_control_master_error_banner =
            *developer_ui.force_show_control_master_error_banner.value();
        let force_show_osc52_clipboard_blocked_banner = *developer_ui
            .force_show_osc52_clipboard_blocked_banner
            .value();
        let force_show_ssh_file_upload = *developer_ui.force_show_ssh_file_upload.value();
        let force_show_first_time_cloud_agent_setup =
            *developer_ui.force_show_first_time_cloud_agent_setup.value();

        let title = Text::new_inline("Developer UI Toggles", appearance.ui_font_family(), 24.)
            .with_style(Properties::default().weight(Weight::Bold))
            .with_color(appearance.theme().active_ui_text_color().into())
            .finish();

        let mut column = Flex::column();
        column.add_child(Container::new(title).with_padding_bottom(20.).finish());

        column.add_child(build_checkbox(
            app,
            "Show Reconnecting Banner",
            force_show_reconnecting_banner,
            self.reconnecting_banner_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleReconnectingBanner,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Slow Bootstrap Banner",
            force_show_slow_bootstrap_banner,
            self.slow_bootstrap_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleSlowBootstrapBanner,
        ));

        column.add_child(build_checkbox(
            app,
            "Show SSH File Drop Overlay",
            force_show_ssh_drop_overlay,
            self.ssh_drop_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleSshDropOverlay,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Jump to Bottom Button",
            force_show_jump_to_bottom,
            self.jump_to_bottom_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleJumpToBottom,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Shared Session Inactivity Modal",
            force_show_inactivity_modal,
            self.inactivity_modal_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleInactivityModal,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Agent Approval UI Card",
            force_show_agent_approval,
            self.agent_approval_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleAgentApproval,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Agent Task/Todo List Card",
            force_show_agent_todos,
            self.agent_todos_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleAgentTodos,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Agent Orchestration Config Blueprint",
            force_show_agent_config,
            self.agent_config_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleAgentConfig,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Agent Interactive Question Card",
            force_show_agent_question,
            self.agent_question_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleAgentQuestion,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Agent Code Diff View",
            force_show_agent_diff,
            self.agent_diff_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleAgentDiff,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Terminal Zero State Dashboard",
            force_show_terminal_zero_state,
            self.terminal_zero_state_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleTerminalZeroState,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Agent Orchestration Zero State Block",
            force_show_agent_zero_state,
            self.agent_zero_state_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleAgentZeroState,
        ));

        column.add_child(build_checkbox(
            app,
            "Show SSH Remote Server Choice View Card",
            force_show_ssh_server_choice,
            self.ssh_server_choice_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleSshServerChoice,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Prompt Suggestions Autocomplete Banner",
            force_show_prompt_suggestions,
            self.prompt_suggestions_mouse.clone(),
            DeveloperUiSettingsPageAction::TogglePromptSuggestions,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Incompatible Configuration Banner",
            force_show_incompatible_configuration_banner,
            self.incompatible_configuration_banner_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleIncompatibleConfigurationBanner,
        ));

        column.add_child(build_checkbox(
            app,
            "Show Emacs Bindings Banner",
            force_show_emacs_bindings_banner,
            self.emacs_bindings_banner_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleEmacsBindingsBanner,
        ));

        column.add_child(build_checkbox(
            app,
            "Show ControlMaster Error Banner",
            force_show_control_master_error_banner,
            self.control_master_error_banner_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleControlMasterErrorBanner,
        ));

        column.add_child(build_checkbox(
            app,
            "Show OSC52 Clipboard Blocked Banner",
            force_show_osc52_clipboard_blocked_banner,
            self.osc52_clipboard_blocked_banner_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleOsc52ClipboardBlockedBanner,
        ));

        column.add_child(build_checkbox(
            app,
            "Show SSH File Upload Panel",
            force_show_ssh_file_upload,
            self.ssh_file_upload_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleSshFileUpload,
        ));

        column.add_child(build_checkbox(
            app,
            "Show First Time Cloud Agent Setup View",
            force_show_first_time_cloud_agent_setup,
            self.first_time_cloud_agent_setup_mouse.clone(),
            DeveloperUiSettingsPageAction::ToggleFirstTimeCloudAgentSetup,
        ));

        Container::new(column.finish())
            .with_padding(Padding::uniform(20.))
            .finish()
    }
}

#[cfg(test)]
#[path = "developer_ui_page_tests.rs"]
mod tests;
