use warpui::elements::{
    Border, ChildView, Container, CornerRadius, CrossAxisAlignment, Flex, MainAxisSize,
    ParentElement, Radius, Shrinkable, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::appearance::Appearance;
use crate::server::server_api::ai::AgentResponse;
use crate::settings::AISettings;
use crate::ui_components::icons::Icon;
use crate::view_components::action_button::{
    ActionButton, ButtonSize, DangerSecondaryTheme, SecondaryTheme,
};

const ACTION_BUTTON_HEIGHT: f32 = 28.;

#[derive(Debug, Clone)]
pub enum CustomAgentViewAction {
    Edit,
    Delete,
}

pub enum CustomAgentViewEvent {
    Edit(String),
    Delete(String),
}

pub struct CustomAgentView {
    agent: AgentResponse,
    edit_button: ViewHandle<ActionButton>,
    delete_button: ViewHandle<ActionButton>,
}

impl CustomAgentView {
    pub fn new(agent: &AgentResponse, ctx: &mut ViewContext<Self>) -> Self {
        let is_any_ai_enabled = AISettings::as_ref(ctx).is_any_ai_enabled(ctx);

        let edit_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Edit", SecondaryTheme)
                .with_icon(Icon::Pencil)
                .with_size(ButtonSize::Small)
                .with_height(ACTION_BUTTON_HEIGHT)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CustomAgentViewAction::Edit);
                })
        });
        edit_button.update(ctx, |button, ctx| {
            button.set_disabled(!is_any_ai_enabled, ctx);
        });

        let delete_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Delete", DangerSecondaryTheme)
                .with_icon(Icon::Trash)
                .with_size(ButtonSize::Small)
                .with_height(ACTION_BUTTON_HEIGHT)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CustomAgentViewAction::Delete);
                })
        });
        delete_button.update(ctx, |button, ctx| {
            button.set_disabled(!is_any_ai_enabled, ctx);
        });

        ctx.subscribe_to_model(&AISettings::handle(ctx), |me, _, _, ctx| {
            let is_any_ai_enabled = AISettings::as_ref(ctx).is_any_ai_enabled(ctx);
            me.edit_button.update(ctx, |button, ctx| {
                button.set_disabled(!is_any_ai_enabled, ctx);
            });
            me.delete_button.update(ctx, |button, ctx| {
                button.set_disabled(!is_any_ai_enabled, ctx);
            });
            ctx.notify();
        });

        Self {
            agent: agent.clone(),
            edit_button,
            delete_button,
        }
    }
}

impl Entity for CustomAgentView {
    type Event = CustomAgentViewEvent;
}

impl TypedActionView for CustomAgentView {
    type Action = CustomAgentViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CustomAgentViewAction::Edit => {
                ctx.emit(CustomAgentViewEvent::Edit(self.agent.uid.clone()));
            }
            CustomAgentViewAction::Delete => {
                ctx.emit(CustomAgentViewEvent::Delete(self.agent.uid.clone()));
            }
        }
    }
}

impl View for CustomAgentView {
    fn ui_name() -> &'static str {
        "CustomAgentView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        let text_color = if is_any_ai_enabled {
            theme.active_ui_text_color()
        } else {
            theme.disabled_ui_text_color()
        };
        let description_color = if is_any_ai_enabled {
            theme.sub_text_color(theme.surface_2())
        } else {
            theme.disabled_ui_text_color()
        };

        let name = Text::new(self.agent.name.clone(), appearance.ui_font_family(), 14.)
            .with_style(Properties::default().weight(Weight::Medium))
            .with_color(text_color.into())
            .finish();
        let description = Text::new(
            self.agent
                .description
                .clone()
                .unwrap_or_else(|| "No description".to_string()),
            appearance.ui_font_family(),
            12.,
        )
        .with_color(description_color.into())
        .soft_wrap(true)
        .finish();

        let details = Flex::column()
            .with_spacing(4.)
            .with_child(name)
            .with_child(description)
            .finish();
        let actions = Flex::row()
            .with_spacing(8.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(ChildView::new(&self.edit_button).finish())
            .with_child(ChildView::new(&self.delete_button).finish())
            .finish();

        let row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Shrinkable::new(1., details).finish())
            .with_child(Container::new(actions).with_margin_left(16.).finish())
            .finish();

        Container::new(row)
            .with_background(theme.surface_2())
            .with_border(Border::all(1.).with_border_fill(theme.outline()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .with_horizontal_padding(16.)
            .with_vertical_padding(12.)
            .finish()
    }
}
