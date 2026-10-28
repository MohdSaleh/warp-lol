use warpui::elements::{
    ChildView, Container, CrossAxisAlignment, Element, Flex, MainAxisAlignment, ParentElement, Text,
};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{AppContext, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};
use warpui_core::Entity;

use crate::appearance::{Appearance, AppearanceEvent};
use crate::editor::{
    EditorOptions, EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys,
    SingleLineEditorOptions, TextOptions,
};
use crate::modal::{Modal, ModalViewState};
use crate::server::server_api::ai::{AgentResponse, CreateAgentRequest, UpdateAgentRequest};
use crate::view_components::action_button::{ActionButton, PrimaryTheme, SecondaryTheme};

const LABEL_FONT_SIZE: f32 = 12.;
const FIELD_LABEL_GAP: f32 = 4.;
const FIELD_GAP: f32 = 16.;
const INSTRUCTIONS_INPUT_HEIGHT: f32 = 144.;
const ACTIONS_TOP_MARGIN: f32 = 24.;

#[derive(Debug, Clone)]
pub enum CustomAgentModalEvent {
    Close,
    Submit,
    Create(CreateAgentRequest),
    Update(String, UpdateAgentRequest),
}

pub struct CustomAgentModal {
    name_editor: ViewHandle<EditorView>,
    description_editor: ViewHandle<EditorView>,
    instructions_editor: ViewHandle<EditorView>,
    save_button: ViewHandle<ActionButton>,
    cancel_button: ViewHandle<ActionButton>,
    editing_agent_uid: Option<String>,
}

impl Entity for CustomAgentModal {
    type Event = CustomAgentModalEvent;
}

impl TypedActionView for CustomAgentModal {
    type Action = CustomAgentModalEvent;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        if matches!(action, CustomAgentModalEvent::Submit) {
            let name = self
                .name_editor
                .read(ctx, |e: &EditorView, app| e.buffer_text(app));
            let desc = self
                .description_editor
                .read(ctx, |e: &EditorView, app| e.buffer_text(app));
            let inst = self
                .instructions_editor
                .read(ctx, |e: &EditorView, app| e.buffer_text(app));
            if name.trim().is_empty() {
                return; // Basic validation
            }

            let desc_opt = if desc.trim().is_empty() {
                None
            } else {
                Some(desc.trim().to_string())
            };

            let inst_opt = if inst.trim().is_empty() {
                None
            } else {
                Some(inst.trim().to_string())
            };

            if let Some(uid) = &self.editing_agent_uid {
                ctx.emit(CustomAgentModalEvent::Update(
                    uid.clone(),
                    UpdateAgentRequest {
                        name: Some(name.trim().to_string()),
                        description: desc_opt,
                        instructions: inst_opt,
                        ..Default::default()
                    },
                ));
            } else {
                ctx.emit(CustomAgentModalEvent::Create(CreateAgentRequest {
                    name: name.trim().to_string(),
                    description: desc_opt,
                    instructions: inst_opt,
                    secrets: vec![],
                    skills: vec![],
                    base_model: None,
                    environment_id: None,
                }));
            }
        } else {
            ctx.emit(action.clone());
        }
    }
}

impl CustomAgentModal {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        // Editor colors are snapshotted at construction, so keep them in sync
        // with the active Warp theme while the modal remains alive.
        ctx.subscribe_to_model(&Appearance::handle(ctx), |me, _, event, ctx| {
            if let AppearanceEvent::ThemeChanged = event {
                me.update_editor_text_colors(ctx);
            }
        });

        let font_family = Appearance::as_ref(ctx).ui_font_family();
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));

        let name_text_colors = text_colors.clone();
        let name_editor = ctx.add_typed_action_view(|ctx| {
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(name_text_colors.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("e.g. My Custom Agent", ctx);
            editor
        });

        let description_text_colors = text_colors.clone();
        let description_editor = ctx.add_typed_action_view(|ctx| {
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(description_text_colors.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("Description (optional)", ctx);
            editor
        });

        let instructions_text_colors = text_colors.clone();
        let instructions_editor = ctx.add_typed_action_view(|ctx| {
            let options = EditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(instructions_text_colors.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                soft_wrap: true,
                // The shared text-input wrapper constrains this editor to a
                // fixed height. With autogrow disabled, EditorView scrolls its
                // own viewport as content exceeds that height.
                autogrow: false,
                ..Default::default()
            };
            let mut editor = EditorView::new(options, ctx);
            editor.set_placeholder_text("System Instructions (optional)", ctx);
            editor
        });

        let save_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Create Agent", PrimaryTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(CustomAgentModalEvent::Submit);
            })
        });
        save_button.update(ctx, |button, ctx| {
            button.set_disabled(true, ctx);
        });

        ctx.subscribe_to_view(&name_editor, |me, _, event, ctx| {
            if matches!(event, EditorEvent::Edited(_) | EditorEvent::BufferReplaced) {
                me.update_save_button_state(ctx);
            }
        });

        let cancel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Cancel", SecondaryTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(CustomAgentModalEvent::Close);
            })
        });

        Self {
            name_editor,
            description_editor,
            instructions_editor,
            save_button,
            cancel_button,
            editing_agent_uid: None,
        }
    }

    fn update_save_button_state(&self, ctx: &mut ViewContext<Self>) {
        let name_is_empty = self
            .name_editor
            .as_ref(ctx)
            .buffer_text(ctx)
            .trim()
            .is_empty();
        self.save_button.update(ctx, |button, ctx| {
            button.set_disabled(name_is_empty, ctx);
        });
    }

    fn update_editor_text_colors(&self, ctx: &mut ViewContext<Self>) {
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));
        for editor in [
            &self.name_editor,
            &self.description_editor,
            &self.instructions_editor,
        ] {
            let colors = text_colors.clone();
            editor.update(ctx, move |editor, ctx| {
                editor.set_text_colors(colors, ctx);
            });
        }
    }

    pub fn prefill(&mut self, agent: Option<&AgentResponse>, ctx: &mut ViewContext<Self>) {
        if let Some(agent) = agent {
            self.save_button.update(ctx, |btn: &mut ActionButton, ctx| {
                btn.set_label("Save", ctx);
            });
            self.editing_agent_uid = Some(agent.uid.clone());
            self.name_editor
                .update(ctx, |editor: &mut EditorView, ctx| {
                    editor.set_buffer_text(&agent.name, ctx);
                });
            self.description_editor
                .update(ctx, |editor: &mut EditorView, ctx| {
                    editor.set_buffer_text(agent.description.as_deref().unwrap_or(""), ctx);
                });
            self.instructions_editor
                .update(ctx, |editor: &mut EditorView, ctx| {
                    editor.set_buffer_text(agent.instructions.as_deref().unwrap_or(""), ctx);
                });
        } else {
            self.save_button.update(ctx, |btn: &mut ActionButton, ctx| {
                btn.set_label("Create Agent", ctx);
            });
            self.editing_agent_uid = None;
            self.name_editor
                .update(ctx, |editor: &mut EditorView, ctx| {
                    editor.set_buffer_text("", ctx);
                });
            self.description_editor
                .update(ctx, |editor: &mut EditorView, ctx| {
                    editor.set_buffer_text("", ctx);
                });
            self.instructions_editor
                .update(ctx, |editor: &mut EditorView, ctx| {
                    editor.set_buffer_text("", ctx);
                });
        }
        self.update_save_button_state(ctx);
    }
}

impl View for CustomAgentModal {
    fn ui_name() -> &'static str {
        "CustomAgentModal"
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(ctx);
        let theme = appearance.theme();

        let name_label = Text::new_inline(
            "Name (required)",
            appearance.ui_font_family(),
            LABEL_FONT_SIZE,
        )
        .with_color(theme.active_ui_text_color().into())
        .finish();

        let description_label =
            Text::new_inline("Description", appearance.ui_font_family(), LABEL_FONT_SIZE)
                .with_color(theme.active_ui_text_color().into())
                .finish();

        let instructions_label = Text::new_inline(
            "System Instructions",
            appearance.ui_font_family(),
            LABEL_FONT_SIZE,
        )
        .with_color(theme.active_ui_text_color().into())
        .finish();

        let name_input = appearance
            .ui_builder()
            .text_input(self.name_editor.clone())
            .build()
            .finish();

        let description_input = appearance
            .ui_builder()
            .text_input(self.description_editor.clone())
            .build()
            .finish();

        let instructions_input = appearance
            .ui_builder()
            .text_input(self.instructions_editor.clone())
            .with_style(UiComponentStyles {
                height: Some(INSTRUCTIONS_INPUT_HEIGHT),
                ..Default::default()
            })
            .build()
            .finish();

        let mut button_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(12.);

        button_row.add_child(Container::new(ChildView::new(&self.cancel_button).finish()).finish());

        button_row.add_child(Container::new(ChildView::new(&self.save_button).finish()).finish());

        let form = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(FIELD_GAP)
            .with_child(
                Flex::column()
                    .with_spacing(FIELD_LABEL_GAP)
                    .with_child(name_label)
                    .with_child(name_input)
                    .finish(),
            )
            .with_child(
                Flex::column()
                    .with_spacing(FIELD_LABEL_GAP)
                    .with_child(description_label)
                    .with_child(description_input)
                    .finish(),
            )
            .with_child(
                Flex::column()
                    .with_spacing(FIELD_LABEL_GAP)
                    .with_child(instructions_label)
                    .with_child(instructions_input)
                    .finish(),
            )
            .with_child(
                Container::new(button_row.finish())
                    .with_margin_top(ACTIONS_TOP_MARGIN - FIELD_GAP)
                    .finish(),
            )
            .finish();

        Container::new(form).finish()
    }
}

pub struct CustomAgentModalViewState {
    state: ModalViewState<Modal<CustomAgentModal>>,
}

impl CustomAgentModalViewState {
    pub fn new(state: ModalViewState<Modal<CustomAgentModal>>) -> Self {
        Self { state }
    }

    pub fn view(&self) -> &ViewHandle<Modal<CustomAgentModal>> {
        &self.state.view
    }

    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }

    pub fn render(&self) -> Box<dyn Element> {
        self.state.render()
    }

    pub fn open<T: View>(&mut self, ctx: &mut ViewContext<T>) {
        self.state.open();
        ctx.notify();
    }

    pub fn close<T: View>(&mut self, ctx: &mut ViewContext<T>) {
        self.state.close();
        ctx.notify();
    }

    pub fn prefill<T: View>(&mut self, agent: Option<&AgentResponse>, ctx: &mut ViewContext<T>) {
        let title = if agent.is_some() {
            "Edit Agent".to_string()
        } else {
            "Create Agent".to_string()
        };
        self.state
            .view
            .update(ctx, |modal: &mut Modal<CustomAgentModal>, ctx| {
                modal.set_title(Some(title));
                modal
                    .body()
                    .update(ctx, |body: &mut CustomAgentModal, ctx| {
                        body.prefill(agent, ctx);
                    });
            });
    }
}

#[cfg(test)]
#[path = "custom_agent_modal_tests.rs"]
mod tests;
