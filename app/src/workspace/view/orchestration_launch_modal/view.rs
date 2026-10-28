use pathfinder_color::ColorU;
use warp_core::ui::theme::Fill;
use warpui::elements::{
    Align, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Empty, Expanded,
    Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement, Radius,
    Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::keymap::FixedBinding;
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::appearance::Appearance;
use crate::server::server_api::ai::AgentResponse;
use crate::server::server_api::ServerApiProvider;
use crate::ui_components::icons::Icon;
use crate::view_components::action_button::{
    ActionButton, ActionButtonTheme, ButtonSize, PrimaryTheme, SecondaryTheme,
};

const MODAL_WIDTH: f32 = 560.;
const MAX_MODAL_HEIGHT: f32 = 520.;

fn modal_background(appearance: &Appearance) -> Fill {
    appearance.theme().surface_3()
}

fn modal_text_main(appearance: &Appearance) -> ColorU {
    appearance
        .theme()
        .main_text_color(modal_background(appearance))
        .into_solid()
}

fn modal_text_sub(appearance: &Appearance) -> ColorU {
    appearance
        .theme()
        .sub_text_color(modal_background(appearance))
        .into_solid()
}

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;
    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        OrchestrationLaunchModalAction::Close,
        id!(OrchestrationLaunchModal::ui_name()),
    )]);
}

// ---------------------------------------------------------------------------
// Actions / Events
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum OrchestrationLaunchModalAction {
    Close,
    FetchAgents,
    AgentsFetched(Result<Vec<AgentResponse>, String>),
    AddAgentToPipeline(usize),
    RemoveAgentFromPipeline(usize),
    MoveAgentUp(usize),
    MoveAgentDown(usize),
    ClearPipeline,
    StartPipeline,
}

#[derive(Clone, Debug)]
pub enum OrchestrationLaunchModalEvent {
    Close,
    StartPipeline(Vec<AgentResponse>),
}

// ---------------------------------------------------------------------------
// Close-button theme (X button in top-right, following AutoHandoffSleepModal)
// ---------------------------------------------------------------------------

struct CloseButtonTheme;

impl ActionButtonTheme for CloseButtonTheme {
    fn background(&self, hovered: bool, appearance: &Appearance) -> Option<Fill> {
        if hovered {
            Some(appearance.theme().surface_overlay_1())
        } else {
            None
        }
    }

    fn text_color(
        &self,
        _hovered: bool,
        _background: Option<Fill>,
        _appearance: &Appearance,
    ) -> ColorU {
        ColorU::white()
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct OrchestrationLaunchModal {
    // Footer buttons
    cancel_button: ViewHandle<ActionButton>,
    start_button: ViewHandle<ActionButton>,
    clear_button: ViewHandle<ActionButton>,
    // Header close
    close_x_button: ViewHandle<ActionButton>,

    // Dynamic buttons for available agents list
    add_buttons: Vec<ViewHandle<ActionButton>>,
    // Dynamic buttons per pipeline slot: remove, up, down
    remove_buttons: Vec<ViewHandle<ActionButton>>,
    move_up_buttons: Vec<ViewHandle<ActionButton>>,
    move_down_buttons: Vec<ViewHandle<ActionButton>>,

    // Hover states for pipeline & available-agent rows
    pipeline_row_mouse_states: Vec<MouseStateHandle>,
    available_row_mouse_states: Vec<MouseStateHandle>,

    available_agents: Option<Result<Vec<AgentResponse>, String>>,
    pipeline: Vec<AgentResponse>,
}

impl Entity for OrchestrationLaunchModal {
    type Event = OrchestrationLaunchModalEvent;
}

impl OrchestrationLaunchModal {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let cancel_button = ctx.add_view(|_ctx| {
            ActionButton::new("Cancel", SecondaryTheme)
                .with_size(ButtonSize::Default)
                .on_click(|ctx| ctx.dispatch_typed_action(OrchestrationLaunchModalAction::Close))
        });

        let start_button = ctx.add_view(|_ctx| {
            ActionButton::new("Start Pipeline", PrimaryTheme)
                .with_size(ButtonSize::Default)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(OrchestrationLaunchModalAction::StartPipeline)
                })
        });

        let clear_button = ctx.add_view(|_ctx| {
            ActionButton::new("Clear All", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(OrchestrationLaunchModalAction::ClearPipeline)
                })
        });

        let close_x_button = ctx.add_view(|_ctx| {
            ActionButton::new("", CloseButtonTheme)
                .with_icon(Icon::X)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| ctx.dispatch_typed_action(OrchestrationLaunchModalAction::Close))
        });

        ctx.dispatch_typed_action(&OrchestrationLaunchModalAction::FetchAgents);

        Self {
            cancel_button,
            start_button,
            clear_button,
            close_x_button,
            add_buttons: Vec::new(),
            remove_buttons: Vec::new(),
            move_up_buttons: Vec::new(),
            move_down_buttons: Vec::new(),
            pipeline_row_mouse_states: Vec::new(),
            available_row_mouse_states: Vec::new(),
            available_agents: None,
            pipeline: Vec::new(),
        }
    }

    /// Ensure enough dynamic buttons exist for the current pipeline and agent
    /// list lengths. Buttons capture their index at creation time; after
    /// removals / reorders the stale buttons are discarded and rebuilt.
    fn rebuild_dynamic_buttons(&mut self, ctx: &mut ViewContext<Self>) {
        // --- Available agents ---
        if let Some(Ok(agents)) = &self.available_agents {
            let needed = agents.len();
            self.add_buttons.clear();
            self.available_row_mouse_states.clear();
            for idx in 0..needed {
                let btn = ctx.add_view(move |_ctx| {
                    ActionButton::new("Add", SecondaryTheme)
                        .with_size(ButtonSize::Small)
                        .on_click(move |ctx| {
                            ctx.dispatch_typed_action(
                                OrchestrationLaunchModalAction::AddAgentToPipeline(idx),
                            )
                        })
                });
                self.add_buttons.push(btn);
                self.available_row_mouse_states
                    .push(MouseStateHandle::default());
            }
        }

        // --- Pipeline slots ---
        let pipe_len = self.pipeline.len();
        self.remove_buttons.clear();
        self.move_up_buttons.clear();
        self.move_down_buttons.clear();
        self.pipeline_row_mouse_states.clear();
        for idx in 0..pipe_len {
            let remove = ctx.add_view(move |_ctx| {
                ActionButton::new("", SecondaryTheme)
                    .with_icon(Icon::X)
                    .with_size(ButtonSize::Small)
                    .on_click(move |ctx| {
                        ctx.dispatch_typed_action(
                            OrchestrationLaunchModalAction::RemoveAgentFromPipeline(idx),
                        )
                    })
            });
            let up = ctx.add_view(move |_ctx| {
                ActionButton::new("", SecondaryTheme)
                    .with_icon(Icon::ChevronUp)
                    .with_size(ButtonSize::Small)
                    .on_click(move |ctx| {
                        ctx.dispatch_typed_action(OrchestrationLaunchModalAction::MoveAgentUp(idx))
                    })
            });
            let down = ctx.add_view(move |_ctx| {
                ActionButton::new("", SecondaryTheme)
                    .with_icon(Icon::ChevronDown)
                    .with_size(ButtonSize::Small)
                    .on_click(move |ctx| {
                        ctx.dispatch_typed_action(OrchestrationLaunchModalAction::MoveAgentDown(
                            idx,
                        ))
                    })
            });
            self.remove_buttons.push(remove);
            self.move_up_buttons.push(up);
            self.move_down_buttons.push(down);
            self.pipeline_row_mouse_states
                .push(MouseStateHandle::default());
        }
    }
}

// ---------------------------------------------------------------------------
// Action handling
// ---------------------------------------------------------------------------

impl TypedActionView for OrchestrationLaunchModal {
    type Action = OrchestrationLaunchModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            OrchestrationLaunchModalAction::Close => {
                ctx.emit(OrchestrationLaunchModalEvent::Close);
            }
            OrchestrationLaunchModalAction::FetchAgents => {
                let ai_client = ServerApiProvider::handle(ctx).as_ref(ctx).get_ai_client();
                ctx.spawn(
                    async move { ai_client.list_agents().await.map_err(|e| e.to_string()) },
                    |_me, res, ctx| {
                        ctx.dispatch_typed_action(&OrchestrationLaunchModalAction::AgentsFetched(
                            res,
                        ));
                    },
                );
            }
            OrchestrationLaunchModalAction::AgentsFetched(res) => {
                self.available_agents = Some(res.clone());
                self.rebuild_dynamic_buttons(ctx);
                ctx.notify();
            }
            OrchestrationLaunchModalAction::AddAgentToPipeline(idx) => {
                if let Some(Ok(agents)) = &self.available_agents {
                    if let Some(agent) = agents.get(*idx) {
                        self.pipeline.push(agent.clone());
                        self.rebuild_dynamic_buttons(ctx);
                        ctx.notify();
                    }
                }
            }
            OrchestrationLaunchModalAction::RemoveAgentFromPipeline(idx) => {
                if *idx < self.pipeline.len() {
                    self.pipeline.remove(*idx);
                    self.rebuild_dynamic_buttons(ctx);
                    ctx.notify();
                }
            }
            OrchestrationLaunchModalAction::MoveAgentUp(idx) => {
                if *idx > 0 && *idx < self.pipeline.len() {
                    self.pipeline.swap(*idx, *idx - 1);
                    self.rebuild_dynamic_buttons(ctx);
                    ctx.notify();
                }
            }
            OrchestrationLaunchModalAction::MoveAgentDown(idx) => {
                if *idx + 1 < self.pipeline.len() {
                    self.pipeline.swap(*idx, *idx + 1);
                    self.rebuild_dynamic_buttons(ctx);
                    ctx.notify();
                }
            }
            OrchestrationLaunchModalAction::ClearPipeline => {
                self.pipeline.clear();
                self.rebuild_dynamic_buttons(ctx);
                ctx.notify();
            }
            OrchestrationLaunchModalAction::StartPipeline => {
                if !self.pipeline.is_empty() {
                    ctx.emit(OrchestrationLaunchModalEvent::StartPipeline(
                        self.pipeline.clone(),
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Render helpers
// ---------------------------------------------------------------------------

impl OrchestrationLaunchModal {
    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        let title = Text::new("Pipeline Builder", appearance.ui_font_family(), 18.)
            .with_color(modal_text_main(appearance))
            .with_style(Properties::default().weight(Weight::Semibold))
            .finish();

        let subtitle = Text::new(
            "Define the sequence of custom agents to run. Each agent runs after the previous one completes.",
            appearance.ui_font_family(),
            13.,
        )
        .with_color(modal_text_sub(appearance))
        .soft_wrap(true)
        .finish();

        let close_x = ChildView::new(&self.close_x_button).finish();

        let title_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Expanded::new(1., title).finish())
            .with_child(close_x)
            .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(8.)
            .with_child(title_row)
            .with_child(subtitle)
            .finish()
    }

    fn render_section_heading(
        label: &str,
        count: usize,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let heading = Text::new(label.to_string(), appearance.ui_font_family(), 13.)
            .with_color(modal_text_sub(appearance))
            .with_style(Properties::default().weight(Weight::Medium))
            .finish();

        let badge = Text::new(format!("{count}"), appearance.ui_font_family(), 11.)
            .with_color(modal_text_sub(appearance))
            .finish();

        let badge_container = Container::new(badge)
            .with_horizontal_padding(6.)
            .with_vertical_padding(2.)
            .with_background(appearance.theme().surface_overlay_1())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
            .finish();

        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(heading)
            .with_child(badge_container)
            .finish()
    }

    fn render_pipeline_section(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let bg = modal_background(appearance);

        let header_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    Self::render_section_heading("PIPELINE", self.pipeline.len(), appearance),
                )
                .finish(),
            )
            .with_child(if self.pipeline.is_empty() {
                Empty::new().finish()
            } else {
                ChildView::new(&self.clear_button).finish()
            })
            .finish();

        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.);

        col.add_child(header_row);

        if self.pipeline.is_empty() {
            let empty_msg = Container::new(
                Text::new(
                    "No agents added yet. Select agents below to build your pipeline.",
                    appearance.ui_font_family(),
                    13.,
                )
                .with_color(modal_text_sub(appearance))
                .finish(),
            )
            .with_vertical_padding(16.)
            .with_horizontal_padding(12.)
            .with_background(theme.surface_overlay_1())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .finish();
            col.add_child(empty_msg);
        } else {
            for (i, agent) in self.pipeline.iter().enumerate() {
                let step_num = Text::new(format!("{}", i + 1), appearance.ui_font_family(), 12.)
                    .with_color(modal_text_sub(appearance))
                    .with_style(Properties::default().weight(Weight::Bold))
                    .finish();

                let step_badge = Container::new(step_num)
                    .with_horizontal_padding(6.)
                    .with_vertical_padding(2.)
                    .with_background(theme.surface_overlay_2())
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                    .finish();

                let name = Text::new(agent.name.clone(), appearance.ui_font_family(), 13.)
                    .with_color(modal_text_main(appearance))
                    .with_style(Properties::default().weight(Weight::Medium))
                    .soft_wrap(false)
                    .finish();

                let mut controls = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(2.);

                if i > 0 {
                    controls.add_child(ChildView::new(&self.move_up_buttons[i]).finish());
                }
                if i + 1 < self.pipeline.len() {
                    controls.add_child(ChildView::new(&self.move_down_buttons[i]).finish());
                }
                controls.add_child(ChildView::new(&self.remove_buttons[i]).finish());

                let mouse_state = self.pipeline_row_mouse_states[i].clone();
                let row = Hoverable::new(mouse_state, move |state| {
                    let row_bg = if state.is_hovered() {
                        theme.surface_overlay_2()
                    } else {
                        theme.surface_overlay_1()
                    };
                    Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_spacing(8.)
                            .with_child(step_badge)
                            .with_child(Expanded::new(1., name).finish())
                            .with_child(controls.finish())
                            .finish(),
                    )
                    .with_horizontal_padding(10.)
                    .with_vertical_padding(6.)
                    .with_background(row_bg)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
                    .finish()
                })
                .finish();

                col.add_child(row);
            }
        }

        col.finish()
    }

    fn render_available_section(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();

        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(4.);

        match &self.available_agents {
            None => {
                col.add_child(Self::render_section_heading(
                    "AVAILABLE AGENTS",
                    0,
                    appearance,
                ));
                col.add_child(
                    Container::new(
                        Text::new("Loading agents…", appearance.ui_font_family(), 13.)
                            .with_color(modal_text_sub(appearance))
                            .finish(),
                    )
                    .with_vertical_padding(12.)
                    .with_horizontal_padding(12.)
                    .finish(),
                );
            }
            Some(Err(e)) => {
                col.add_child(Self::render_section_heading(
                    "AVAILABLE AGENTS",
                    0,
                    appearance,
                ));
                col.add_child(
                    Container::new(
                        Text::new(
                            format!("Failed to load agents: {e}"),
                            appearance.ui_font_family(),
                            13.,
                        )
                        .with_color(modal_text_sub(appearance))
                        .finish(),
                    )
                    .with_vertical_padding(12.)
                    .with_horizontal_padding(12.)
                    .finish(),
                );
            }
            Some(Ok(agents)) => {
                col.add_child(Self::render_section_heading(
                    "AVAILABLE AGENTS",
                    agents.len(),
                    appearance,
                ));

                if agents.is_empty() {
                    col.add_child(
                        Container::new(
                            Text::new(
                                "No custom agents found. Create one in Settings → AI.",
                                appearance.ui_font_family(),
                                13.,
                            )
                            .with_color(modal_text_sub(appearance))
                            .finish(),
                        )
                        .with_vertical_padding(12.)
                        .with_horizontal_padding(12.)
                        .with_background(theme.surface_overlay_1())
                        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
                        .finish(),
                    );
                } else {
                    for (i, agent) in agents.iter().enumerate() {
                        let name = Text::new(agent.name.clone(), appearance.ui_font_family(), 13.)
                            .with_color(modal_text_main(appearance))
                            .with_style(Properties::default().weight(Weight::Medium))
                            .soft_wrap(false)
                            .finish();

                        let desc_text = agent
                            .description
                            .clone()
                            .unwrap_or_else(|| "No description".to_string());
                        let desc = Text::new(desc_text, appearance.ui_font_family(), 12.)
                            .with_color(modal_text_sub(appearance))
                            .soft_wrap(false)
                            .finish();

                        let info_col = Flex::column()
                            .with_spacing(2.)
                            .with_child(name)
                            .with_child(desc)
                            .finish();

                        let add_btn = ChildView::new(&self.add_buttons[i]).finish();

                        let mouse_state = self.available_row_mouse_states[i].clone();
                        let row = Hoverable::new(mouse_state, move |state| {
                            let row_bg = if state.is_hovered() {
                                theme.surface_overlay_2()
                            } else {
                                theme.surface_overlay_1()
                            };
                            Container::new(
                                Flex::row()
                                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                    .with_spacing(8.)
                                    .with_child(Expanded::new(1., info_col).finish())
                                    .with_child(add_btn)
                                    .finish(),
                            )
                            .with_horizontal_padding(10.)
                            .with_vertical_padding(6.)
                            .with_background(row_bg)
                            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
                            .finish()
                        })
                        .finish();

                        col.add_child(row);
                    }
                }
            }
        }

        col.finish()
    }

    fn render_footer(&self, appearance: &Appearance) -> Box<dyn Element> {
        let status = if self.pipeline.is_empty() {
            Text::new(
                "Add at least one agent to start",
                appearance.ui_font_family(),
                12.,
            )
            .with_color(modal_text_sub(appearance))
            .finish()
        } else {
            Text::new(
                format!(
                    "{} agent{} in pipeline",
                    self.pipeline.len(),
                    if self.pipeline.len() == 1 { "" } else { "s" }
                ),
                appearance.ui_font_family(),
                12.,
            )
            .with_color(modal_text_main(appearance))
            .finish()
        };

        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Expanded::new(1., status).finish())
            .with_spacing(12.)
            .with_child(ChildView::new(&self.cancel_button).finish())
            .with_child(ChildView::new(&self.start_button).finish())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// View impl
// ---------------------------------------------------------------------------

impl View for OrchestrationLaunchModal {
    fn ui_name() -> &'static str {
        "OrchestrationLaunchModal"
    }

    fn on_focus(&mut self, _focus_ctx: &warpui::FocusContext, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        let body = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(20.)
            .with_child(self.render_header(appearance))
            .with_child(self.render_pipeline_section(appearance))
            .with_child(self.render_available_section(appearance))
            .with_child(self.render_footer(appearance))
            .finish();

        let card = ConstrainedBox::new(
            Container::new(body)
                .with_horizontal_padding(28.)
                .with_vertical_padding(24.)
                .with_background(modal_background(appearance))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(12.)))
                .finish(),
        )
        .with_width(MODAL_WIDTH)
        .with_max_height(MAX_MODAL_HEIGHT)
        .finish();

        // Full-screen backdrop that closes on click
        Container::new(Align::new(card).finish())
            .with_background(Fill::Solid(ColorU::new(0, 0, 0, 128)))
            .finish()
    }
}
