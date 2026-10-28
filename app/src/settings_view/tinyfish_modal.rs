use ai::api_keys::ApiKeyManager;
use warpui::elements::{
    ChildView, Container, CrossAxisAlignment, Element, Flex, MainAxisAlignment, ParentElement,
};
use warpui::prelude::Padding;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};
use warpui_core::scene::{Border, CornerRadius, Radius};

use crate::appearance::Appearance;
use crate::editor::{EditorView, SingleLineEditorOptions};
use crate::view_components::action_button::{ActionButton, PrimaryTheme, SecondaryTheme};

pub enum TinyFishModalEvent {
    Close,
    Save(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TinyFishModalAction {
    Cancel,
    Save,
}

pub struct TinyFishModalBody {
    editor: ViewHandle<EditorView>,
    cancel_button: ViewHandle<ActionButton>,
    save_button: ViewHandle<ActionButton>,
}

impl TinyFishModalBody {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let manager = ApiKeyManager::as_ref(ctx);
        let current_key = manager.keys().tinyfish.clone().unwrap_or_default();

        let editor = ctx.add_view(|ctx| {
            let editor_opts = SingleLineEditorOptions {
                is_password: true,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(editor_opts, ctx);
            editor.set_buffer_text(&current_key, ctx);
            editor.set_placeholder_text("Enter TinyFish API Key", ctx);
            editor
        });

        let cancel_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Cancel", SecondaryTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(TinyFishModalAction::Cancel);
            })
        });

        let save_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Save", PrimaryTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(TinyFishModalAction::Save);
            })
        });

        Self {
            editor,
            cancel_button,
            save_button,
        }
    }
}

impl Entity for TinyFishModalBody {
    type Event = TinyFishModalEvent;
}

impl View for TinyFishModalBody {
    fn ui_name() -> &'static str {
        "TinyFishModalBody"
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(ctx);
        let theme = appearance.theme();

        let label = appearance
            .ui_builder()
            .span("API Key".to_string())
            .with_style(UiComponentStyles {
                font_size: Some(14.),
                ..Default::default()
            })
            .build()
            .finish();

        let input_row = Flex::column()
            .with_child(Container::new(label).with_margin_bottom(6.).finish())
            .with_child(
                Container::new(ChildView::new(&self.editor).finish())
                    .with_uniform_padding(8.)
                    .with_background(theme.surface_2())
                    .with_border(Border::all(1.).with_border_fill(theme.outline()))
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
                    .finish(),
            )
            .finish();

        let form = Flex::column()
            .with_child(input_row)
            .with_child(
                Container::new(
                    Flex::row()
                        .with_child(ChildView::new(&self.cancel_button).finish())
                        .with_child(ChildView::new(&self.save_button).finish())
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_main_axis_alignment(MainAxisAlignment::End)
                        .with_spacing(12.)
                        .finish(),
                )
                .with_margin_top(24.)
                .finish(),
            )
            .finish();

        Container::new(form)
            .with_padding(Padding::uniform(24.))
            .finish()
    }
}

impl TypedActionView for TinyFishModalBody {
    type Action = TinyFishModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            TinyFishModalAction::Cancel => {
                ctx.emit(TinyFishModalEvent::Close);
            }
            TinyFishModalAction::Save => {
                let key = self.editor.as_ref(ctx).buffer_text(ctx);
                ctx.emit(TinyFishModalEvent::Save(key));
            }
        }
    }
}
