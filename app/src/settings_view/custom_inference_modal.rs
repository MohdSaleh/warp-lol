use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ::ai::api_keys::CustomEndpoint;
use ::ai::provider_catalog::{ProviderUrlKind, PROVIDER_CATALOG};
use url::Url;
use warp_editor::editor::NavigationKey;
use warpui::elements::{
    Border, ChildView, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container,
    CornerRadius, CrossAxisAlignment, Empty, Expanded, Flex, MainAxisSize, MouseStateHandle,
    ParentElement, Radius, SavePosition, ScrollTarget, ScrollToPositionMode, ScrollbarWidth,
    Shrinkable, Text,
};
use warpui::fonts::FamilyId;
use warpui::ui_components::button::ButtonVariant;
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::units::Pixels;
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::appearance::{Appearance, AppearanceEvent};
use crate::editor::{
    EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys, SingleLineEditorOptions,
    TextOptions,
};
use crate::modal::{Modal, ModalViewState};
use crate::ui_components::icons::Icon;
use crate::view_components::action_button::{ActionButton, DangerSecondaryTheme};

const LABEL_FONT_SIZE: f32 = 12.;
const INPUT_WIDTH: f32 = 480.;
const ENDPOINT_NAME_SCROLL_POSITION_ID: &str = "custom_endpoint_name";
const ENDPOINT_URL_SCROLL_POSITION_ID: &str = "custom_endpoint_url";
const API_KEY_SCROLL_POSITION_ID: &str = "custom_endpoint_api_key";
const ACTIONS_POSITION_ID: &str = "custom_endpoint_actions";

const MODEL_ROW_SPACING: f32 = 16.;
const REMOVE_MODEL_BUTTON_SPACING: f32 = 8.;
const REMOVE_MODEL_BUTTON_COL_WIDTH: f32 = 20.;
const MODAL_SCROLLBAR_WIDTH: f32 = 4.;
// Right margin shared by the scrollable content and the (non-scrollable) actions
// row. It includes the overlayed scrollbar width so the scrollable content's right
// edge (e.g. the remove-model "X" buttons) lines up with the Save/Cancel buttons,
// which have no scrollbar.
const SCROLL_CONTENT_RIGHT_MARGIN: f32 = 24. + MODAL_SCROLLBAR_WIDTH;
const MODEL_INPUT_WIDTH: f32 = (INPUT_WIDTH - MODEL_ROW_SPACING) / 2.;
fn model_row_scroll_position_id(index: usize) -> String {
    format!("custom_endpoint_model_row_{index}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomEndpointModalEvent {
    Close,
    AddEndpoint {
        name: String,
        url: String,
        api_key: String,
        models: Vec<(String, Option<String>, Option<String>)>,
        /// The catalog preset id this endpoint was created from, if any.
        provider_id: Option<String>,
    },
    SaveEndpoint {
        index: usize,
        name: String,
        url: String,
        api_key: String,
        models: Vec<(String, Option<String>, Option<String>)>,
    },
    RemoveEndpoint {
        index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomEndpointModalAction {
    Cancel,
    Save,
    AddModel,
    RemoveModel(usize),
    RemoveEndpoint,
    /// Prefill the form from `PROVIDER_CATALOG[index]`.
    ApplyPreset(usize),
    /// Fetch the provider's available models from its `/v1/models` endpoint.
    LoadModels,
    /// Toggle selection of a fetched model in the pick-list.
    ToggleFetchedModel(usize),
}

struct ModelRow {
    name_editor: ViewHandle<EditorView>,
    alias_editor: ViewHandle<EditorView>,
    remove_mouse_state: MouseStateHandle,
    config_key: Option<String>,
}

/// One model returned by the provider's `/v1/models` endpoint, with its
/// selected state and click target for the pick-list.
struct FetchedModelRow {
    id: String,
    selected: bool,
    /// Click target for the row; only read by the native render path.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    mouse_state: MouseStateHandle,
}

pub struct CustomEndpointModal {
    endpoint_name_editor: ViewHandle<EditorView>,
    endpoint_url_editor: ViewHandle<EditorView>,
    api_key_editor: ViewHandle<EditorView>,
    model_rows: Vec<ModelRow>,
    cancel_button_mouse_state: MouseStateHandle,
    save_button_mouse_state: MouseStateHandle,
    add_model_button_mouse_state: MouseStateHandle,
    remove_endpoint_button: ViewHandle<ActionButton>,
    editing_index: Option<usize>,
    url_has_error: bool,
    scroll_state: ClippedScrollStateHandle,
    /// The catalog preset the form was populated from, if any. Carried onto the
    /// created endpoint so the settings list can show provider branding.
    selected_provider_id: Option<String>,
    /// One mouse state per catalog preset button (parallel to `PROVIDER_CATALOG`).
    preset_button_states: Vec<MouseStateHandle>,

    /// "Load models" button click target. (Only read on native, where the
    /// fetch is available.)
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    load_models_button_mouse_state: MouseStateHandle,
    /// True while a `/v1/models` fetch is in flight.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    models_loading: bool,
    /// Last fetch error, surfaced under the button.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    models_error: Option<String>,
    /// Incremented whenever a model-list fetch is started or invalidated so
    /// stale async responses cannot repopulate the picker after URL/key edits.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    models_load_generation: u64,
    /// Models returned by the provider, selectable by the user.
    available_models: Vec<FetchedModelRow>,
}

impl CustomEndpointModal {
    pub fn new(
        endpoint: Option<&CustomEndpoint>,
        editing_index: Option<usize>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        // Editor text colors are snapshotted at construction via
        // `text_colors_override`, so refresh them whenever the theme changes.
        ctx.subscribe_to_model(&Appearance::handle(ctx), |me, _, event, ctx| {
            if let AppearanceEvent::ThemeChanged = event {
                me.update_editor_text_colors(ctx);
            }
        });
        let font_family = Appearance::as_ref(ctx).ui_font_family();
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));

        let endpoint_name_text_colors = text_colors.clone();
        let endpoint_name_editor = ctx.add_typed_action_view(move |ctx| {
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(endpoint_name_text_colors.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("e.g., Zach's external models", ctx);
            if let Some(ep) = endpoint {
                editor.set_buffer_text(&ep.name, ctx);
            }
            editor
        });

        let endpoint_url_text_colors = text_colors.clone();
        let endpoint_url_editor = ctx.add_typed_action_view(move |ctx| {
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(endpoint_url_text_colors.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("Please include 'https://'", ctx);
            if let Some(ep) = endpoint {
                editor.set_buffer_text(&ep.url, ctx);
            }
            editor
        });

        let api_key_text_colors = text_colors.clone();
        let api_key_editor = ctx.add_typed_action_view(move |ctx| {
            let options = SingleLineEditorOptions {
                is_password: true,
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(api_key_text_colors.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("e.g., sk-...", ctx);
            if let Some(ep) = endpoint {
                editor.set_buffer_text(&ep.api_key, ctx);
            }
            editor
        });

        let mut model_rows = Vec::new();
        if let Some(ep) = endpoint {
            for model in &ep.models {
                model_rows.push(Self::create_model_row(
                    Some(&model.name),
                    model.alias.as_deref(),
                    Some(model.config_key.clone()),
                    font_family,
                    &text_colors,
                    ctx,
                ));
            }
        }
        if model_rows.is_empty() {
            model_rows.push(Self::create_model_row(
                None,
                None,
                None,
                font_family,
                &text_colors,
                ctx,
            ));
        }

        ctx.subscribe_to_view(&endpoint_name_editor, |me, _, event, ctx| {
            me.handle_endpoint_name_event(event, ctx);
        });
        ctx.subscribe_to_view(&endpoint_url_editor, |me, _, event, ctx| {
            me.handle_endpoint_url_event(event, ctx);
        });
        // Validate initial URL (if any) so the error state is accurate on open.
        let initial_url = endpoint_url_editor.as_ref(ctx).buffer_text(ctx);
        let url_has_error = !initial_url.trim().is_empty()
            && validate_url(&initial_url, crate::ai::local_byok::is_local_profile(ctx)).is_err();
        ctx.subscribe_to_view(&api_key_editor, |me, _, event, ctx| {
            me.handle_api_key_event(event, ctx);
        });
        for row in &model_rows {
            let name_editor = row.name_editor.clone();
            ctx.subscribe_to_view(&name_editor, |me, editor, event, ctx| {
                me.handle_model_editor_event(&editor, event, ctx);
            });
            let alias_editor = row.alias_editor.clone();
            ctx.subscribe_to_view(&alias_editor, |me, editor, event, ctx| {
                me.handle_model_editor_event(&editor, event, ctx);
            });
        }
        let remove_endpoint_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Remove", DangerSecondaryTheme)
                .with_icon(Icon::Trash)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(CustomEndpointModalAction::RemoveEndpoint);
                })
        });

        Self {
            endpoint_name_editor,
            endpoint_url_editor,
            api_key_editor,
            model_rows,
            cancel_button_mouse_state: Default::default(),
            save_button_mouse_state: Default::default(),
            add_model_button_mouse_state: Default::default(),
            remove_endpoint_button,
            editing_index,
            url_has_error,
            scroll_state: Default::default(),
            selected_provider_id: endpoint.and_then(|ep| ep.provider_id.clone()),
            preset_button_states: PROVIDER_CATALOG
                .iter()
                .map(|_| MouseStateHandle::default())
                .collect(),
            load_models_button_mouse_state: Default::default(),
            models_loading: false,
            models_error: None,
            models_load_generation: 0,
            available_models: Vec::new(),
        }
    }

    /// Fetches the provider's `/v1/models` list and populates the pick-list, so
    /// the user selects models instead of hand-typing slugs.
    #[cfg(not(target_family = "wasm"))]
    fn load_models(&mut self, ctx: &mut ViewContext<Self>) {
        self.models_load_generation = self.models_load_generation.wrapping_add(1);
        let generation = self.models_load_generation;
        let url = self.endpoint_url_editor.as_ref(ctx).buffer_text(ctx);
        if url.trim().is_empty()
            || validate_url(&url, crate::ai::local_byok::is_local_profile(ctx)).is_err()
        {
            self.models_error = Some("Enter a valid endpoint URL first.".to_string());
            ctx.notify();
            return;
        }
        let api_key = self.api_key_editor.as_ref(ctx).buffer_text(ctx);
        if api_key.trim().is_empty() {
            self.models_error = Some("Enter an API key first.".to_string());
            ctx.notify();
            return;
        }
        self.models_loading = true;
        self.models_error = None;
        ctx.notify();
        ctx.spawn(
            async move { ::ai::provider_models::fetch_models(&url, &api_key).await },
            move |me, result, ctx| {
                if me.models_load_generation != generation {
                    return;
                }
                me.models_loading = false;
                match result {
                    Ok(ids) => {
                        // Preserve any prior selections across a re-fetch.
                        let previously_selected: HashSet<String> = me
                            .available_models
                            .iter()
                            .filter(|m| m.selected)
                            .map(|m| m.id.clone())
                            .collect();
                        me.available_models = ids
                            .into_iter()
                            .map(|id| {
                                let selected = previously_selected.contains(&id);
                                FetchedModelRow {
                                    id,
                                    selected,
                                    mouse_state: MouseStateHandle::default(),
                                }
                            })
                            .collect();
                        me.models_error = None;
                    }
                    Err(err) => {
                        me.models_error = Some(format!("{err}"));
                    }
                }
                ctx.notify();
            },
        );
    }

    fn toggle_fetched_model(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if let Some(model) = self.available_models.get_mut(index) {
            model.selected = !model.selected;
            ctx.notify();
        }
    }

    fn invalidate_loaded_models(&mut self) {
        self.models_load_generation = self.models_load_generation.wrapping_add(1);
        self.models_loading = false;
        self.models_error = None;
        self.available_models.clear();
    }

    /// Populates the form fields from a catalog preset, so connecting a known
    /// provider is "pick provider, paste key".
    fn apply_preset(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        let Some(preset) = PROVIDER_CATALOG.get(index) else {
            return;
        };
        self.selected_provider_id = Some(preset.id.to_string());
        // A new provider invalidates any previously fetched model list.
        self.invalidate_loaded_models();
        self.endpoint_name_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(preset.display_name, ctx);
        });
        self.endpoint_url_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(preset.base_url, ctx);
        });
        // Leave the API key untouched so re-picking doesn't wipe a pasted key.
        self.api_key_editor.update(ctx, |editor, ctx| {
            editor.set_placeholder_text(preset.key_placeholder, ctx);
        });
        let url = self.endpoint_url_editor.as_ref(ctx).buffer_text(ctx);
        self.url_has_error = !url.trim().is_empty()
            && validate_url(&url, crate::ai::local_byok::is_local_profile(ctx)).is_err();

        // Rebuild model rows from the preset's suggested models.
        self.model_rows.clear();
        let font_family = Appearance::as_ref(ctx).ui_font_family();
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));
        for (slug, alias) in preset.suggested_models {
            self.model_rows.push(Self::create_model_row(
                Some(*slug),
                (!alias.is_empty()).then_some(*alias),
                None,
                font_family,
                &text_colors,
                ctx,
            ));
        }
        if self.model_rows.is_empty() {
            self.model_rows.push(Self::create_model_row(
                None,
                None,
                None,
                font_family,
                &text_colors,
                ctx,
            ));
        }
        for row in &self.model_rows {
            let name_editor = row.name_editor.clone();
            ctx.subscribe_to_view(&name_editor, |me, editor, event, ctx| {
                me.handle_model_editor_event(&editor, event, ctx);
            });
            let alias_editor = row.alias_editor.clone();
            ctx.subscribe_to_view(&alias_editor, |me, editor, event, ctx| {
                me.handle_model_editor_event(&editor, event, ctx);
            });
        }
        match preset.url_kind {
            ProviderUrlKind::Fixed => self.focus_editor(&self.api_key_editor, ctx),
            ProviderUrlKind::Template | ProviderUrlKind::SelfHosted => {
                self.focus_editor(&self.endpoint_url_editor, ctx);
            }
        }
        ctx.notify();
    }

    fn create_model_row(
        name: Option<&str>,
        alias: Option<&str>,
        config_key: Option<String>,
        font_family: FamilyId,
        text_colors: &crate::editor::TextColors,
        ctx: &mut ViewContext<Self>,
    ) -> ModelRow {
        let tc = text_colors.clone();
        let name_editor = ctx.add_typed_action_view(move |ctx| {
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(tc.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("e.g., GLM-5-FP8", ctx);
            if let Some(n) = name {
                editor.set_buffer_text(n, ctx);
            }
            editor
        });

        let tc = text_colors.clone();
        let alias_editor = ctx.add_typed_action_view(move |ctx| {
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_family_override: Some(font_family),
                    text_colors_override: Some(tc.clone()),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("e.g., GLM-5", ctx);
            if let Some(a) = alias {
                editor.set_buffer_text(a, ctx);
            }
            editor
        });

        ModelRow {
            name_editor,
            alias_editor,
            remove_mouse_state: Default::default(),
            config_key,
        }
    }

    pub fn prefill(
        &mut self,
        endpoint: Option<&CustomEndpoint>,
        editing_index: Option<usize>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.editing_index = editing_index;
        self.scroll_state = Default::default();
        self.selected_provider_id = endpoint.and_then(|ep| ep.provider_id.clone());
        self.invalidate_loaded_models();
        self.endpoint_name_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(endpoint.map(|e| e.name.as_str()).unwrap_or(""), ctx);
        });
        self.endpoint_url_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(endpoint.map(|e| e.url.as_str()).unwrap_or(""), ctx);
        });
        let url = self.endpoint_url_editor.as_ref(ctx).buffer_text(ctx);
        self.url_has_error = !url.trim().is_empty()
            && validate_url(&url, crate::ai::local_byok::is_local_profile(ctx)).is_err();
        self.api_key_editor.update(ctx, |editor, ctx| {
            editor.set_buffer_text(endpoint.map(|e| e.api_key.as_str()).unwrap_or(""), ctx);
        });
        // Rebuild model rows
        // Old model row editors will be dropped with the modal body
        self.model_rows.clear();
        let font_family = Appearance::as_ref(ctx).ui_font_family();
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));
        if let Some(ep) = endpoint {
            for model in &ep.models {
                self.model_rows.push(Self::create_model_row(
                    Some(&model.name),
                    model.alias.as_deref(),
                    Some(model.config_key.clone()),
                    font_family,
                    &text_colors,
                    ctx,
                ));
            }
        }
        if self.model_rows.is_empty() {
            self.model_rows.push(Self::create_model_row(
                None,
                None,
                None,
                font_family,
                &text_colors,
                ctx,
            ));
        }
        for row in &self.model_rows {
            let name_editor = row.name_editor.clone();
            ctx.subscribe_to_view(&name_editor, |me, editor, event, ctx| {
                me.handle_model_editor_event(&editor, event, ctx);
            });
            let alias_editor = row.alias_editor.clone();
            ctx.subscribe_to_view(&alias_editor, |me, editor, event, ctx| {
                me.handle_model_editor_event(&editor, event, ctx);
            });
        }
    }

    pub fn on_open(&mut self, ctx: &mut ViewContext<Self>) {
        self.scroll_state.scroll_to(Pixels::zero());
        self.focus_editor(&self.endpoint_name_editor, ctx);
    }

    pub fn on_close(&mut self, ctx: &mut ViewContext<Self>) {
        self.endpoint_name_editor.update(ctx, |editor, ctx| {
            editor.clear_buffer_and_reset_undo_stack(ctx);
        });
        self.endpoint_url_editor.update(ctx, |editor, ctx| {
            editor.clear_buffer_and_reset_undo_stack(ctx);
        });
        self.api_key_editor.update(ctx, |editor, ctx| {
            editor.clear_buffer_and_reset_undo_stack(ctx);
        });
        for row in &self.model_rows {
            row.name_editor.update(ctx, |editor, ctx| {
                editor.clear_buffer_and_reset_undo_stack(ctx);
            });
            row.alias_editor.update(ctx, |editor, ctx| {
                editor.clear_buffer_and_reset_undo_stack(ctx);
            });
        }
        self.invalidate_loaded_models();
    }

    /// Re-applies theme-derived text colors to every editor in the modal.
    /// Called on appearance changes since editors only snapshot their text
    /// colors at construction.
    fn update_editor_text_colors(&mut self, ctx: &mut ViewContext<Self>) {
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));
        let mut editors = vec![
            self.endpoint_name_editor.clone(),
            self.endpoint_url_editor.clone(),
            self.api_key_editor.clone(),
        ];
        for row in &self.model_rows {
            editors.push(row.name_editor.clone());
            editors.push(row.alias_editor.clone());
        }
        for editor in editors {
            let colors = text_colors.clone();
            editor.update(ctx, move |editor, ctx| {
                editor.set_text_colors(colors, ctx);
            });
        }
    }

    fn save(&mut self, ctx: &mut ViewContext<Self>) {
        self.validate_url_field(ctx);
        if !self.is_valid(ctx) {
            return;
        }
        let name = self.endpoint_name_editor.as_ref(ctx).buffer_text(ctx);
        let url = self.endpoint_url_editor.as_ref(ctx).buffer_text(ctx);
        let api_key = self.api_key_editor.as_ref(ctx).buffer_text(ctx);
        let models = self.selected_models_for_save(ctx);
        if let Some(index) = self.editing_index {
            ctx.emit(CustomEndpointModalEvent::SaveEndpoint {
                index,
                name,
                url,
                api_key,
                models,
            });
        } else {
            ctx.emit(CustomEndpointModalEvent::AddEndpoint {
                name,
                url,
                api_key,
                models,
                provider_id: self.selected_provider_id.clone(),
            });
        }
    }

    fn selected_models_for_save(
        &self,
        ctx: &AppContext,
    ) -> Vec<(String, Option<String>, Option<String>)> {
        let manual_models: Vec<_> = self
            .model_rows
            .iter()
            .filter_map(|row| {
                let name = row.name_editor.as_ref(ctx).buffer_text(ctx);
                if name.trim().is_empty() {
                    return None;
                }
                let alias = row.alias_editor.as_ref(ctx).buffer_text(ctx);
                let alias_opt = if alias.trim().is_empty() {
                    None
                } else {
                    Some(alias)
                };
                Some((name, alias_opt, row.config_key.clone()))
            })
            .collect();

        let mut models = Vec::new();
        if self.available_models.is_empty() {
            models.extend(manual_models);
        } else {
            let manual_by_name: HashMap<String, _> = manual_models
                .iter()
                .cloned()
                .map(|entry| (model_name_key(&entry.0), entry))
                .collect();
            let fetched_names: HashSet<_> = self
                .available_models
                .iter()
                .map(|model| model_name_key(&model.id))
                .collect();

            for model in self.available_models.iter().filter(|model| model.selected) {
                let key = model_name_key(&model.id);
                models.push(
                    manual_by_name
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| (model.id.clone(), None, None)),
                );
            }

            models.extend(
                manual_models
                    .into_iter()
                    .filter(|(name, _, _)| !fetched_names.contains(&model_name_key(name))),
            );
        }

        let mut seen = HashSet::new();
        models.retain(|(name, _, _)| seen.insert(model_name_key(name)));
        models
    }

    fn cancel(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(CustomEndpointModalEvent::Close);
    }

    fn add_model(&mut self, ctx: &mut ViewContext<Self>) {
        let font_family = Appearance::as_ref(ctx).ui_font_family();
        let text_colors = crate::settings_view::editor_text_colors(Appearance::as_ref(ctx));
        let row = Self::create_model_row(None, None, None, font_family, &text_colors, ctx);
        // Subscribe to the new editors
        let name_editor = row.name_editor.clone();
        ctx.subscribe_to_view(&name_editor, |me, editor, event, ctx| {
            me.handle_model_editor_event(&editor, event, ctx);
        });
        let alias_editor = row.alias_editor.clone();
        ctx.subscribe_to_view(&alias_editor, |me, editor, event, ctx| {
            me.handle_model_editor_event(&editor, event, ctx);
        });
        self.model_rows.push(row);
        // ClippedScrollable clamps this to its true maximum after laying out the new row.
        // Until the form overflows, that maximum remains zero and the modal grows naturally.
        self.scroll_state.scroll_to(Pixels::new(f32::MAX));
        ctx.notify();
    }

    fn remove_model(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if index < self.model_rows.len() {
            let _row = self.model_rows.remove(index);
            ctx.notify();
        }
    }

    fn is_valid(&self, app: &AppContext) -> bool {
        let name = self.endpoint_name_editor.as_ref(app).buffer_text(app);
        let url = self.endpoint_url_editor.as_ref(app).buffer_text(app);
        let api_key = self.api_key_editor.as_ref(app).buffer_text(app);
        let has_manual_models = self.model_rows.iter().any(|row| {
            !row.name_editor
                .as_ref(app)
                .buffer_text(app)
                .trim()
                .is_empty()
        });
        let has_selected_fetched = self.available_models.iter().any(|m| m.selected);
        is_endpoint_form_valid(
            &name,
            &url,
            &api_key,
            has_manual_models || has_selected_fetched,
            crate::ai::local_byok::is_local_profile(app),
        )
    }

    fn focus_editor(&self, editor: &ViewHandle<EditorView>, ctx: &mut ViewContext<Self>) {
        ctx.focus(editor);
        let position_id = if self.endpoint_name_editor == *editor {
            Some(ENDPOINT_NAME_SCROLL_POSITION_ID.to_string())
        } else if self.endpoint_url_editor == *editor {
            Some(ENDPOINT_URL_SCROLL_POSITION_ID.to_string())
        } else if self.api_key_editor == *editor {
            Some(API_KEY_SCROLL_POSITION_ID.to_string())
        } else {
            self.model_rows
                .iter()
                .position(|row| row.name_editor == *editor || row.alias_editor == *editor)
                .map(model_row_scroll_position_id)
        };
        if let Some(position_id) = position_id {
            self.scroll_state.scroll_to_position(ScrollTarget {
                position_id,
                mode: ScrollToPositionMode::FullyIntoView,
            });
            ctx.notify();
        }
    }

    fn focus_next_editor(&self, current: &ViewHandle<EditorView>, ctx: &mut ViewContext<Self>) {
        let mut editors: Vec<&ViewHandle<EditorView>> = vec![
            &self.endpoint_name_editor,
            &self.endpoint_url_editor,
            &self.api_key_editor,
        ];
        for row in &self.model_rows {
            editors.push(&row.name_editor);
            editors.push(&row.alias_editor);
        }
        if let Some(pos) = editors.iter().position(|e| *e == current) {
            let next = (pos + 1) % editors.len();
            self.focus_editor(editors[next], ctx);
        }
    }

    fn focus_prev_editor(&self, current: &ViewHandle<EditorView>, ctx: &mut ViewContext<Self>) {
        let mut editors: Vec<&ViewHandle<EditorView>> = vec![
            &self.endpoint_name_editor,
            &self.endpoint_url_editor,
            &self.api_key_editor,
        ];
        for row in &self.model_rows {
            editors.push(&row.name_editor);
            editors.push(&row.alias_editor);
        }
        if let Some(pos) = editors.iter().position(|e| *e == current) {
            let prev = if pos == 0 { editors.len() - 1 } else { pos - 1 };
            self.focus_editor(editors[prev], ctx);
        }
    }

    fn handle_endpoint_name_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        match event {
            EditorEvent::Navigate(NavigationKey::Tab) => {
                self.focus_editor(&self.endpoint_url_editor, ctx);
            }
            EditorEvent::Navigate(NavigationKey::ShiftTab) => {
                self.focus_prev_editor(&self.endpoint_name_editor, ctx);
            }
            EditorEvent::Enter => {
                self.focus_editor(&self.endpoint_url_editor, ctx);
            }
            EditorEvent::Escape => {
                self.cancel(ctx);
            }
            EditorEvent::Edited(_) => {
                ctx.notify();
            }
            _ => {}
        }
    }

    fn handle_endpoint_url_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        match event {
            EditorEvent::Navigate(NavigationKey::Tab) => {
                self.validate_url_field(ctx);
                self.focus_editor(&self.api_key_editor, ctx);
            }
            EditorEvent::Navigate(NavigationKey::ShiftTab) => {
                self.validate_url_field(ctx);
                self.focus_editor(&self.endpoint_name_editor, ctx);
            }
            EditorEvent::Enter => {
                self.validate_url_field(ctx);
                self.focus_editor(&self.api_key_editor, ctx);
            }
            EditorEvent::Escape => {
                self.cancel(ctx);
            }
            EditorEvent::Edited(_) => {
                self.invalidate_loaded_models();
                if !self.validate_url_field(ctx) {
                    ctx.notify();
                }
            }
            _ => {}
        }
    }

    fn validate_url_field(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        let url = self.endpoint_url_editor.as_ref(ctx).buffer_text(ctx);
        let had_error = self.url_has_error;
        self.url_has_error = !url.trim().is_empty()
            && validate_url(&url, crate::ai::local_byok::is_local_profile(ctx)).is_err();
        let changed = self.url_has_error != had_error;
        if changed {
            ctx.notify();
        }
        changed
    }

    fn handle_api_key_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        match event {
            EditorEvent::Navigate(NavigationKey::Tab) => {
                if let Some(first_row) = self.model_rows.first() {
                    self.focus_editor(&first_row.name_editor, ctx);
                }
            }
            EditorEvent::Navigate(NavigationKey::ShiftTab) => {
                self.focus_editor(&self.endpoint_url_editor, ctx);
            }
            EditorEvent::Enter => {
                if let Some(first_row) = self.model_rows.first() {
                    self.focus_editor(&first_row.name_editor, ctx);
                }
            }
            EditorEvent::Escape => {
                self.cancel(ctx);
            }
            EditorEvent::Edited(_) => {
                self.invalidate_loaded_models();
                ctx.notify();
            }
            _ => {}
        }
    }

    fn handle_model_editor_event(
        &mut self,
        editor: &ViewHandle<EditorView>,
        event: &EditorEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            EditorEvent::Navigate(NavigationKey::Tab) | EditorEvent::Enter => {
                self.focus_next_editor(editor, ctx);
            }
            EditorEvent::Navigate(NavigationKey::ShiftTab) => {
                self.focus_prev_editor(editor, ctx);
            }
            EditorEvent::Escape => {
                self.cancel(ctx);
            }
            EditorEvent::Edited(_) => {
                ctx.notify();
            }
            _ => {}
        }
    }
}

impl Entity for CustomEndpointModal {
    type Event = CustomEndpointModalEvent;
}

impl View for CustomEndpointModal {
    fn ui_name() -> &'static str {
        "CustomEndpointModal"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let is_valid = self.is_valid(app);
        let is_editing = self.editing_index.is_some();

        let label_font_family = appearance.ui_font_family();
        let label_text_color = theme.active_ui_text_color().into();
        let label = move |text: &'static str| {
            Text::new(text, label_font_family, LABEL_FONT_SIZE)
                .with_color(label_text_color)
                .finish()
        };

        let input_style = UiComponentStyles {
            width: Some(INPUT_WIDTH),
            ..Default::default()
        };
        let button_style = UiComponentStyles {
            font_size: Some(14.),
            padding: Some(Coords::uniform(8.).left(12.).right(12.)),
            ..Default::default()
        };

        let mut column = Flex::column();

        // Description
        column.add_child(
            Container::new(
                Text::new(
                    "Provide your endpoint details below. You can add as many models from the endpoint as you'd like and can also provide aliases for the model picker in your input.",
                    appearance.ui_font_family(),
                    LABEL_FONT_SIZE,
                )
                .with_color(theme.nonactive_ui_text_color().into())
                .soft_wrap(true)
                .finish(),
            )
            .with_margin_bottom(16.)
            .finish(),
        );

        // Provider preset picker: a wrapping row of clickable chips that
        // pre-fill the form for a known provider. Only shown when adding a new
        // endpoint (not when editing an existing one).
        if self.editing_index.is_none() {
            column.add_child(
                Container::new(label("Quick add a provider"))
                    .with_margin_bottom(6.)
                    .finish(),
            );

            // No wrapping-flex primitive exists, so lay the chips out as a
            // column of fixed-width rows (3 per row within INPUT_WIDTH).
            const PRESETS_PER_ROW: usize = 3;
            const PRESET_ROW_SPACING: f32 = 8.;
            let preset_button_width = (INPUT_WIDTH
                - PRESET_ROW_SPACING * (PRESETS_PER_ROW as f32 - 1.))
                / PRESETS_PER_ROW as f32;
            let mut presets_column = Flex::column().with_spacing(PRESET_ROW_SPACING);
            for chunk in PROVIDER_CATALOG
                .iter()
                .enumerate()
                .collect::<Vec<_>>()
                .chunks(PRESETS_PER_ROW)
            {
                let mut presets_row = Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_spacing(PRESET_ROW_SPACING);
                for (index, preset) in chunk.iter().copied() {
                    let is_selected = self.selected_provider_id.as_deref() == Some(preset.id);
                    let variant = if is_selected {
                        ButtonVariant::Accent
                    } else {
                        ButtonVariant::Secondary
                    };
                    let button = appearance
                        .ui_builder()
                        .button(
                            variant,
                            self.preset_button_states
                                .get(index)
                                .cloned()
                                .unwrap_or_default(),
                        )
                        .with_text_label(preset.display_name.to_string())
                        .with_style(UiComponentStyles {
                            font_size: Some(12.),
                            padding: Some(Coords::uniform(5.).left(8.).right(8.)),
                            ..Default::default()
                        })
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(CustomEndpointModalAction::ApplyPreset(
                                index,
                            ));
                        })
                        .finish();
                    presets_row.add_child(
                        ConstrainedBox::new(button)
                            .with_width(preset_button_width)
                            .finish(),
                    );
                }
                presets_column.add_child(presets_row.finish());
            }
            column.add_child(
                Container::new(presets_column.finish())
                    .with_margin_bottom(16.)
                    .finish(),
            );

            // Surface the selected preset's note/key link, if any.
            if let Some(preset) = self
                .selected_provider_id
                .as_deref()
                .and_then(::ai::provider_catalog::preset_by_id)
            {
                if let Some(note) = preset.note {
                    column.add_child(
                        Container::new(
                            Text::new(note, appearance.ui_font_family(), 11.)
                                .with_color(theme.nonactive_ui_text_color().into())
                                .soft_wrap(true)
                                .finish(),
                        )
                        .with_margin_bottom(6.)
                        .finish(),
                    );
                }
                let is_self_hosted = matches!(preset.url_kind, ProviderUrlKind::SelfHosted);
                let hint = if is_self_hosted {
                    "Enter your server's public HTTPS URL below, then a placeholder key."
                } else {
                    "Get an API key, paste it below, and adjust the models if needed."
                };
                column.add_child(
                    Container::new(
                        Text::new(hint, appearance.ui_font_family(), 11.)
                            .with_color(theme.nonactive_ui_text_color().into())
                            .soft_wrap(true)
                            .finish(),
                    )
                    .with_margin_bottom(16.)
                    .finish(),
                );
            }
        }

        // Endpoint name
        column.add_child(
            Container::new(label("Endpoint name"))
                .with_margin_bottom(4.)
                .finish(),
        );
        column.add_child(
            SavePosition::new(
                Container::new(
                    appearance
                        .ui_builder()
                        .text_input(self.endpoint_name_editor.clone())
                        .with_style(input_style)
                        .build()
                        .finish(),
                )
                .with_margin_bottom(16.)
                .finish(),
                ENDPOINT_NAME_SCROLL_POSITION_ID,
            )
            .finish(),
        );

        // Endpoint URL
        column.add_child(
            Container::new(label("Endpoint URL"))
                .with_margin_bottom(4.)
                .finish(),
        );
        let url_border_fill = if self.url_has_error {
            theme.ui_error_color().into()
        } else {
            theme.outline()
        };
        column.add_child(
            SavePosition::new(
                Container::new(
                    appearance
                        .ui_builder()
                        .text_input(self.endpoint_url_editor.clone())
                        .with_style(input_style)
                        .build()
                        .finish(),
                )
                .with_border(Border::all(1.).with_border_fill(url_border_fill))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                .with_margin_bottom(16.)
                .finish(),
                ENDPOINT_URL_SCROLL_POSITION_ID,
            )
            .finish(),
        );

        // API key
        column.add_child(
            Container::new(label("API key"))
                .with_margin_bottom(4.)
                .finish(),
        );
        column.add_child(
            SavePosition::new(
                Container::new(
                    appearance
                        .ui_builder()
                        .text_input(self.api_key_editor.clone())
                        .with_style(input_style)
                        .build()
                        .finish(),
                )
                .with_margin_bottom(16.)
                .finish(),
                API_KEY_SCROLL_POSITION_ID,
            )
            .finish(),
        );

        // "Load available models" button + fetched pick-list. Lets the user
        // select from the provider's real model list instead of hand-typing
        // slugs. Native-only (the fetch uses `http_client`).
        #[cfg(not(target_family = "wasm"))]
        {
            let load_label = if self.models_loading {
                "Loading models…".to_string()
            } else {
                "Load available models".to_string()
            };
            let mut load_button = appearance
                .ui_builder()
                .button(
                    ButtonVariant::Secondary,
                    self.load_models_button_mouse_state.clone(),
                )
                .with_text_label(load_label)
                .with_style(UiComponentStyles {
                    font_size: Some(13.),
                    padding: Some(Coords::uniform(6.).left(10.).right(10.)),
                    ..Default::default()
                });
            if self.models_loading {
                load_button = load_button.disabled();
            }
            let load_button = load_button
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CustomEndpointModalAction::LoadModels);
                })
                .finish();
            column.add_child(Container::new(load_button).with_margin_bottom(8.).finish());

            if let Some(err) = &self.models_error {
                column.add_child(
                    Container::new(
                        Text::new(err.clone(), appearance.ui_font_family(), 11.)
                            .with_color(theme.ui_error_color())
                            .soft_wrap(true)
                            .finish(),
                    )
                    .with_margin_bottom(8.)
                    .finish(),
                );
            }
        }

        if !self.available_models.is_empty() {
            let selected_count = self.available_models.iter().filter(|m| m.selected).count();
            column.add_child(
                Container::new(
                    Text::new(
                        format!(
                            "Select models to use ({selected_count}/{} selected)",
                            self.available_models.len()
                        ),
                        appearance.ui_font_family(),
                        LABEL_FONT_SIZE,
                    )
                    .with_color(theme.active_ui_text_color().into())
                    .finish(),
                )
                .with_margin_bottom(6.)
                .finish(),
            );
            let mut list = Flex::column().with_spacing(6.);
            for (index, model) in self.available_models.iter().enumerate() {
                let variant = if model.selected {
                    ButtonVariant::Accent
                } else {
                    ButtonVariant::Secondary
                };
                let button = appearance
                    .ui_builder()
                    .button(variant, model.mouse_state.clone())
                    .with_text_label(model.id.clone())
                    .with_style(UiComponentStyles {
                        font_size: Some(12.),
                        padding: Some(Coords::uniform(6.).left(10.).right(10.)),
                        ..Default::default()
                    })
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(CustomEndpointModalAction::ToggleFetchedModel(
                            index,
                        ));
                    })
                    .finish();
                list.add_child(ConstrainedBox::new(button).with_width(INPUT_WIDTH).finish());
            }
            column.add_child(
                Container::new(list.finish())
                    .with_margin_bottom(16.)
                    .finish(),
            );
        }

        // Model rows
        let has_remove_model_button = self.model_rows.len() > 1;
        let mut model_labels = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(REMOVE_MODEL_BUTTON_SPACING)
            .with_child(
                Flex::row()
                    .with_spacing(MODEL_ROW_SPACING)
                    .with_child(
                        ConstrainedBox::new(label("Model name"))
                            .with_width(MODEL_INPUT_WIDTH)
                            .finish(),
                    )
                    .with_child(
                        ConstrainedBox::new(label("Model alias (optional)"))
                            .with_width(MODEL_INPUT_WIDTH)
                            .finish(),
                    )
                    .finish(),
            );
        if has_remove_model_button {
            model_labels.add_child(
                ConstrainedBox::new(Empty::new().finish())
                    .with_width(REMOVE_MODEL_BUTTON_COL_WIDTH)
                    .finish(),
            );
        }

        // Model column labels
        column.add_child(
            Container::new(model_labels.finish())
                .with_margin_bottom(4.)
                .finish(),
        );

        for (i, row) in self.model_rows.iter().enumerate() {
            let name_input = appearance
                .ui_builder()
                .text_input(row.name_editor.clone())
                .with_style(UiComponentStyles {
                    width: Some(MODEL_INPUT_WIDTH),
                    ..Default::default()
                })
                .build()
                .finish();

            let alias_input = appearance
                .ui_builder()
                .text_input(row.alias_editor.clone())
                .with_style(UiComponentStyles {
                    width: Some(MODEL_INPUT_WIDTH),
                    ..Default::default()
                })
                .build()
                .finish();

            let remove_button = if self.model_rows.len() > 1 {
                appearance
                    .ui_builder()
                    .close_button(20., row.remove_mouse_state.clone())
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(CustomEndpointModalAction::RemoveModel(i));
                    })
                    .finish()
            } else {
                Empty::new().finish()
            };
            let model_inputs = Flex::row()
                .with_spacing(MODEL_ROW_SPACING)
                .with_child(name_input)
                .with_child(alias_input)
                .finish();

            let mut row = Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(REMOVE_MODEL_BUTTON_SPACING)
                .with_child(model_inputs);
            if has_remove_model_button {
                row.add_child(
                    ConstrainedBox::new(remove_button)
                        .with_width(REMOVE_MODEL_BUTTON_COL_WIDTH)
                        .finish(),
                );
            }
            let row = row.finish();

            column.add_child(
                SavePosition::new(
                    Container::new(row).with_margin_bottom(12.).finish(),
                    &model_row_scroll_position_id(i),
                )
                .finish(),
            );
        }

        let add_model_button = appearance
            .ui_builder()
            .button(
                ButtonVariant::Secondary,
                self.add_model_button_mouse_state.clone(),
            )
            .with_text_label("+ Add model".to_string())
            .with_style(UiComponentStyles {
                font_size: Some(14.),
                padding: Some(Coords::uniform(6.).left(8.).right(8.)),
                ..Default::default()
            })
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(CustomEndpointModalAction::AddModel);
            })
            .finish();

        column.add_child(
            Container::new(add_model_button)
                .with_margin_bottom(24.)
                .finish(),
        );

        // Bottom buttons row
        let mut buttons_row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);

        // Remove button (only when editing)
        if is_editing {
            buttons_row.add_child(ChildView::new(&self.remove_endpoint_button).finish());
        }

        buttons_row.add_child(Expanded::new(1., Empty::new().finish()).finish());

        buttons_row.add_child(
            appearance
                .ui_builder()
                .button(
                    ButtonVariant::Secondary,
                    self.cancel_button_mouse_state.clone(),
                )
                .with_text_label("Cancel".to_string())
                .with_style(button_style)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(CustomEndpointModalAction::Cancel);
                })
                .finish(),
        );

        let mut save_button = appearance
            .ui_builder()
            .button(ButtonVariant::Accent, self.save_button_mouse_state.clone())
            .with_text_label(if is_editing {
                "Save".to_string()
            } else {
                "Add endpoint".to_string()
            })
            .with_style(button_style);
        if !is_valid {
            save_button = save_button.disabled();
        }

        buttons_row.add_child(
            Container::new(
                save_button
                    .build()
                    .on_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(CustomEndpointModalAction::Save);
                    })
                    .finish(),
            )
            .with_margin_left(12.)
            .finish(),
        );

        let scrollable_content = ClippedScrollable::vertical(
            self.scroll_state.clone(),
            Container::new(column.finish())
                .with_margin_right(SCROLL_CONTENT_RIGHT_MARGIN)
                .finish(),
            ScrollbarWidth::Custom(MODAL_SCROLLBAR_WIDTH),
            theme.nonactive_ui_text_color().into(),
            theme.active_ui_text_color().into(),
            warpui::elements::Fill::None,
        )
        .with_overlayed_scrollbar()
        .with_padding_start(0.)
        .with_padding_end(0.)
        .finish();
        let buttons_row = SavePosition::new(
            Container::new(buttons_row.finish())
                .with_margin_right(SCROLL_CONTENT_RIGHT_MARGIN)
                .finish(),
            ACTIONS_POSITION_ID,
        )
        .for_single_frame()
        .finish();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(Shrinkable::new(1., scrollable_content).finish())
            .with_child(buttons_row)
            .finish()
    }
}

fn validate_url(url: &str, is_local_mode: bool) -> Result<(), &'static str> {
    if url.trim().is_empty() {
        return Ok(());
    }
    let parsed = Url::parse(url).map_err(|_| "Invalid URL")?;
    let Some(host) = parsed.host_str().filter(|h| !h.is_empty()) else {
        return Err("URL must include a host");
    };

    let is_localhost = host.eq_ignore_ascii_case("localhost")
        || host.parse::<std::net::IpAddr>().is_ok_and(|ip| match ip {
            std::net::IpAddr::V4(ip) => ip.is_loopback(),
            std::net::IpAddr::V6(ip) => ip.is_loopback(),
        });

    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && is_localhost && is_local_mode) {
        return Err("URL must use HTTPS (except for localhost in Local Mode)");
    }
    if is_restricted_host(host) && !(is_local_mode && is_localhost) {
        return Err("URL must not use a local or private host");
    }
    Ok(())
}

fn is_endpoint_form_valid(
    name: &str,
    url: &str,
    api_key: &str,
    has_models: bool,
    is_local_mode: bool,
) -> bool {
    !name.trim().is_empty()
        && !url.trim().is_empty()
        && !api_key.trim().is_empty()
        && has_models
        && validate_url(url, is_local_mode).is_ok()
}

fn model_name_key(name: &str) -> String {
    name.trim().to_string()
}

fn is_restricted_host(host: &str) -> bool {
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<IpAddr>().is_ok_and(is_restricted_ip)
}

fn is_restricted_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_restricted_ipv4(ip),
        IpAddr::V6(ip) => is_restricted_ipv6(ip),
    }
}

fn is_restricted_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_unspecified() || ip.is_private() || ip.is_link_local()
}

fn is_restricted_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || is_ipv6_unique_local(ip) || is_ipv6_link_local(ip)
    {
        return true;
    }
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return is_restricted_ipv4(ipv4);
    }
    false
}

fn is_ipv6_unique_local(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xfe00 == 0xfc00
}

fn is_ipv6_link_local(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xffc0 == 0xfe80
}
impl TypedActionView for CustomEndpointModal {
    type Action = CustomEndpointModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            CustomEndpointModalAction::Cancel => self.cancel(ctx),
            CustomEndpointModalAction::Save => self.save(ctx),
            CustomEndpointModalAction::AddModel => self.add_model(ctx),
            CustomEndpointModalAction::RemoveModel(index) => self.remove_model(*index, ctx),
            CustomEndpointModalAction::RemoveEndpoint => {
                if let Some(index) = self.editing_index {
                    ctx.emit(CustomEndpointModalEvent::RemoveEndpoint { index });
                }
            }
            CustomEndpointModalAction::ApplyPreset(index) => self.apply_preset(*index, ctx),
            CustomEndpointModalAction::LoadModels => {
                #[cfg(not(target_family = "wasm"))]
                self.load_models(ctx);
            }
            CustomEndpointModalAction::ToggleFetchedModel(index) => {
                self.toggle_fetched_model(*index, ctx);
            }
        }
    }
}

#[cfg(test)]
#[path = "custom_inference_modal_tests.rs"]
mod tests;

pub struct CustomEndpointModalViewState {
    state: ModalViewState<Modal<CustomEndpointModal>>,
}

impl CustomEndpointModalViewState {
    pub fn new(state: ModalViewState<Modal<CustomEndpointModal>>) -> Self {
        Self { state }
    }

    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }

    pub fn render(&self) -> Box<dyn Element> {
        self.state.render()
    }

    pub fn set_title<T: View>(&mut self, title: Option<String>, ctx: &mut ViewContext<T>) {
        self.state.view.update(ctx, |modal, ctx| {
            modal.set_title(title);
            ctx.notify();
        });
    }

    pub fn prefill<T: View>(
        &mut self,
        endpoint: Option<&CustomEndpoint>,
        editing_index: Option<usize>,
        ctx: &mut ViewContext<T>,
    ) {
        self.state.view.update(ctx, |modal, ctx| {
            modal.body().update(ctx, |body, ctx| {
                body.prefill(endpoint, editing_index, ctx);
            });
        });
    }

    pub fn open<T: View>(&mut self, ctx: &mut ViewContext<T>) {
        self.state.open();
        self.state.view.update(ctx, |modal, ctx| {
            modal.body().update(ctx, |body, ctx| {
                body.on_open(ctx);
            });
        });
    }

    pub fn close<T: View>(&mut self, ctx: &mut ViewContext<T>) {
        self.state.close();
        self.state.view.update(ctx, |modal, ctx| {
            modal.body().update(ctx, |body, ctx| {
                body.on_close(ctx);
            });
        });
    }
}
