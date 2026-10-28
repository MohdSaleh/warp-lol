use ai::api_keys::CustomEndpointModel;
use pathfinder_geometry::vector::vec2f;
use warpui::platform::WindowStyle;
use warpui::scene::Scene;
use warpui::units::Pixels;
use warpui::{App, EntityIdSet, Presenter, WindowInvalidation};

use super::*;
use crate::test_util::terminal::initialize_app_for_terminal_view;

fn endpoint_with_models(model_count: usize) -> CustomEndpoint {
    CustomEndpoint {
        name: "Test endpoint".to_string(),
        url: "https://api.example.com/v1".to_string(),
        api_key: "key".to_string(),
        models: (0..model_count)
            .map(|index| CustomEndpointModel {
                name: format!("model-{index}"),
                alias: None,
                config_key: format!("config-{index}"),
            })
            .collect(),
        provider_id: None,
    }
}

fn fetched_model(id: &str, selected: bool) -> FetchedModelRow {
    FetchedModelRow {
        id: id.to_string(),
        selected,
        mouse_state: MouseStateHandle::default(),
    }
}

fn init_modal_test_models(app: &mut App) {
    initialize_app_for_terminal_view(app);
}
fn custom_endpoint_modal_height(scene: &Scene) -> f32 {
    let rects = scene
        .layers()
        .flat_map(|layer| &layer.rects)
        .map(|rect| (rect.bounds.width(), rect.bounds.height(), rect.border.width))
        .collect::<Vec<_>>();
    rects
        .iter()
        .filter(|(width, _, _)| *width > INPUT_WIDTH && *width <= 560.)
        .map(|(_, height, _)| *height)
        .max_by(f32::total_cmp)
        .unwrap_or_else(|| panic!("custom endpoint modal rect should exist: {rects:?}"))
}

#[test]
fn modal_resizes_with_window_and_added_models() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let endpoint = endpoint_with_models(1);
        let (window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            let body = ctx.add_typed_action_view(|ctx| {
                CustomEndpointModal::new(Some(&endpoint), Some(0), ctx)
            });
            Modal::new(Some("Edit custom endpoint".to_string()), body, ctx)
                .with_modal_style(UiComponentStyles {
                    width: Some(560.),
                    ..Default::default()
                })
                .with_max_height_percentage(0.8)
        });
        let body = modal.read(&app, |modal, _| modal.body().clone());
        let mut presenter = Presenter::new(window_id);
        let invalidation = WindowInvalidation {
            updated: EntityIdSet::from_iter([
                app.root_view_id(window_id).expect("root view should exist"),
                body.id(),
            ]),
            ..Default::default()
        };

        app.update(move |ctx| {
            presenter.invalidate(invalidation.clone(), ctx);
            let initial_modal_height = {
                let scene = presenter.build_scene(vec2f(800., 1000.), 1., None, ctx);
                custom_endpoint_modal_height(&scene)
            };
            body.update(ctx, |body, ctx| {
                for _ in 0..20 {
                    body.add_model(ctx);
                }
            });
            presenter.invalidate(invalidation, ctx);
            let expanded_modal_height = {
                let scene = presenter.build_scene(vec2f(800., 1000.), 1., None, ctx);
                custom_endpoint_modal_height(&scene)
            };
            let small_window_height = {
                let scene = presenter.build_scene(vec2f(800., 500.), 1., None, ctx);
                custom_endpoint_modal_height(&scene)
            };

            assert!(
                expanded_modal_height > initial_modal_height,
                "expanded modal height {expanded_modal_height} should be greater than initial modal height {initial_modal_height}"
            );
            assert!(
                (expanded_modal_height - 765.).abs() < 0.1,
                "expanded modal height {expanded_modal_height} should reach the 80% window-height cap"
            );
            assert!(
                small_window_height < expanded_modal_height,
                "small modal height {small_window_height} should be less than expanded modal height {expanded_modal_height}"
            );
            assert!(
                (small_window_height - 365.).abs() < 0.1,
                "small modal height {small_window_height} should reach the 80% window-height cap"
            );
        });
    })
}

#[test]
fn modal_with_many_models_lays_out() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let endpoint = endpoint_with_models(20);
        let (window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            CustomEndpointModal::new(Some(&endpoint), Some(0), ctx)
        });

        app.update(|ctx| {
            ctx.presenter(window_id)
                .expect("presenter should exist")
                .borrow_mut()
                .build_scene(vec2f(560., 600.), 1., None, ctx);

            assert_eq!(modal.as_ref(ctx).model_rows.len(), 20);
        });
    })
}

#[test]
fn model_row_inputs_align_and_controls_fit_gutter() {
    assert_eq!(MODEL_INPUT_WIDTH * 2. + MODEL_ROW_SPACING, INPUT_WIDTH);
    // SCROLL_CONTENT_RIGHT_MARGIN already includes MODAL_SCROLLBAR_WIDTH, so the
    // right gutter (button spacing + remove-button column + content right margin)
    // is 56 without adding the scrollbar width again.
    assert_eq!(
        REMOVE_MODEL_BUTTON_SPACING + REMOVE_MODEL_BUTTON_COL_WIDTH + SCROLL_CONTENT_RIGHT_MARGIN,
        56.
    );
}

#[test]
fn action_row_remains_fixed_when_form_scrolls() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let endpoint = endpoint_with_models(20);
        let (window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            let body = ctx.add_typed_action_view(|ctx| {
                CustomEndpointModal::new(Some(&endpoint), Some(0), ctx)
            });
            Modal::new(Some("Edit custom endpoint".to_string()), body, ctx)
                .with_modal_style(UiComponentStyles {
                    width: Some(560.),
                    ..Default::default()
                })
                .with_max_height_percentage(0.8)
        });
        let body = modal.read(&app, |modal, _| modal.body().clone());
        let invalidation = WindowInvalidation {
            updated: EntityIdSet::from_iter([
                app.root_view_id(window_id).expect("root view should exist"),
                body.id(),
            ]),
            ..Default::default()
        };

        let action_row_position = app.update(|ctx| {
            let presenter = ctx.presenter(window_id).expect("presenter should exist");
            let mut presenter = presenter.borrow_mut();
            presenter.invalidate(invalidation.clone(), ctx);
            presenter.build_scene(vec2f(560., 600.), 1., None, ctx);
            presenter
                .position_cache()
                .get_position(ACTIONS_POSITION_ID)
                .expect("action row position should exist")
        });
        body.update(&mut app, |body, ctx| {
            body.scroll_state.scroll_to(Pixels::new(f32::MAX));
            ctx.notify();
        });

        let scrolled_action_row_position = app.update(|ctx| {
            let presenter = ctx.presenter(window_id).expect("presenter should exist");
            let mut presenter = presenter.borrow_mut();
            presenter.invalidate(invalidation, ctx);
            presenter.build_scene(vec2f(560., 600.), 1., None, ctx);
            presenter
                .position_cache()
                .get_position(ACTIONS_POSITION_ID)
                .expect("action row position should exist")
        });
        assert!(body.read(&app, |body, _| body.scroll_state.scroll_start()) > Pixels::zero());
        assert_eq!(
            action_row_position, scrolled_action_row_position,
            "action row should remain fixed while form content scrolls"
        );
    })
}
#[test]
fn focus_editor_scrolls_whole_form_to_field() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let endpoint = endpoint_with_models(20);
        let (window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            CustomEndpointModal::new(Some(&endpoint), Some(0), ctx)
        });

        app.update(|ctx| {
            ctx.presenter(window_id)
                .expect("presenter should exist")
                .borrow_mut()
                .build_scene(vec2f(560., 600.), 1., None, ctx);
        });
        modal.update(&mut app, |modal, ctx| {
            let editor = modal
                .model_rows
                .last()
                .expect("model row should exist")
                .name_editor
                .clone();
            modal.focus_editor(&editor, ctx);
        });
        app.update(|ctx| {
            ctx.presenter(window_id)
                .expect("presenter should exist")
                .borrow_mut()
                .build_scene(vec2f(560., 600.), 1., None, ctx);
            assert!(modal.as_ref(ctx).scroll_state.scroll_start() > Pixels::zero());
        });
        let model_scroll_start = modal.read(&app, |modal, _| modal.scroll_state.scroll_start());
        modal.update(&mut app, |modal, ctx| {
            modal.focus_editor(&modal.endpoint_name_editor.clone(), ctx);
        });
        app.update(|ctx| {
            ctx.presenter(window_id)
                .expect("presenter should exist")
                .borrow_mut()
                .build_scene(vec2f(560., 600.), 1., None, ctx);
            assert!(modal.as_ref(ctx).scroll_state.scroll_start() < model_scroll_start);
        });
    })
}

#[test]
fn add_model_scrolls_only_after_form_is_full() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let endpoint = endpoint_with_models(1);
        let (window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            CustomEndpointModal::new(Some(&endpoint), Some(0), ctx)
        });

        modal.update(&mut app, |modal, ctx| modal.add_model(ctx));
        app.update(|ctx| {
            ctx.presenter(window_id)
                .expect("presenter should exist")
                .borrow_mut()
                .build_scene(vec2f(560., 600.), 1., None, ctx);

            assert_eq!(
                modal.as_ref(ctx).scroll_state.scroll_start(),
                Pixels::zero()
            );
        });

        modal.update(&mut app, |modal, ctx| {
            for _ in 0..20 {
                modal.add_model(ctx);
            }
            assert_eq!(modal.scroll_state.scroll_start(), Pixels::new(f32::MAX));
        });
        app.update(|ctx| {
            ctx.presenter(window_id)
                .expect("presenter should exist")
                .borrow_mut()
                .build_scene(vec2f(560., 600.), 1., None, ctx);
            let scroll_start = modal.as_ref(ctx).scroll_state.scroll_start();
            assert!(scroll_start > Pixels::zero());
            assert!(scroll_start < Pixels::new(f32::MAX));
        });
    })
}

#[test]
fn prefill_resets_form_scroll_position() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let endpoint = endpoint_with_models(20);
        let (_window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            CustomEndpointModal::new(Some(&endpoint), Some(0), ctx)
        });

        modal.update(&mut app, |modal, ctx| {
            modal.scroll_state.scroll_to(Pixels::new(100.));
            assert_eq!(modal.scroll_state.scroll_start(), Pixels::new(100.));

            modal.prefill(None, None, ctx);

            assert_eq!(modal.scroll_state.scroll_start(), Pixels::zero());
        });
    })
}

#[test]
fn selected_models_for_save_uses_fetched_selection_and_preserves_manual_metadata() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let (_window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            CustomEndpointModal::new(None, None, ctx)
        });

        modal.update(&mut app, |modal, ctx| {
            let first_row = modal
                .model_rows
                .first_mut()
                .expect("model row should exist");
            first_row.config_key = Some("config-a".to_string());
            first_row.name_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text("model-a", ctx);
            });
            first_row.alias_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text("Alias A", ctx);
            });

            modal.add_model(ctx);
            let second_row = modal
                .model_rows
                .get(1)
                .expect("second model row should exist");
            second_row.name_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text("manual-only", ctx);
            });

            modal.available_models = vec![
                fetched_model("model-a", true),
                fetched_model("model-b", false),
                fetched_model("model-c", true),
            ];

            assert_eq!(
                modal.selected_models_for_save(ctx),
                vec![
                    (
                        "model-a".to_string(),
                        Some("Alias A".to_string()),
                        Some("config-a".to_string())
                    ),
                    ("model-c".to_string(), None, None),
                    ("manual-only".to_string(), None, None),
                ]
            );
        });
    })
}

#[test]
fn editing_url_or_api_key_invalidates_loaded_models() {
    App::test((), |mut app| async move {
        init_modal_test_models(&mut app);
        let (_window_id, modal) = app.add_window(WindowStyle::NotStealFocus, move |ctx| {
            CustomEndpointModal::new(None, None, ctx)
        });

        modal.update(&mut app, |modal, ctx| {
            modal.available_models = vec![fetched_model("model-a", true)];
            modal.models_error = Some("old error".to_string());
            modal.models_loading = true;
            let initial_generation = modal.models_load_generation;

            modal.handle_endpoint_url_event(
                &EditorEvent::Edited(crate::editor::EditOrigin::UserTyped),
                ctx,
            );

            assert!(modal.available_models.is_empty());
            assert!(modal.models_error.is_none());
            assert!(!modal.models_loading);
            assert!(modal.models_load_generation > initial_generation);

            modal.available_models = vec![fetched_model("model-b", true)];
            modal.models_error = Some("old error".to_string());
            modal.models_loading = true;
            let generation_after_url_edit = modal.models_load_generation;

            modal.handle_api_key_event(
                &EditorEvent::Edited(crate::editor::EditOrigin::UserTyped),
                ctx,
            );

            assert!(modal.available_models.is_empty());
            assert!(modal.models_error.is_none());
            assert!(!modal.models_loading);
            assert!(modal.models_load_generation > generation_after_url_edit);
        });
    })
}

#[test]
fn validate_url_accepts_https_with_host() {
    assert!(validate_url("https://api.example.com/v1", false).is_ok());
    assert!(validate_url("https://example.com", false).is_ok());
    assert!(validate_url("https://8.8.8.8/v1", false).is_ok());
}

#[test]
fn validate_url_rejects_http() {
    assert_eq!(
        validate_url("http://api.example.com/v1", false),
        Err("URL must use HTTPS (except for localhost in Local Mode)")
    );
    assert_eq!(
        validate_url("http://example.com", false),
        Err("URL must use HTTPS (except for localhost in Local Mode)")
    );
}

#[test]
fn validate_url_rejects_ftp_and_other_schemes() {
    assert_eq!(
        validate_url("ftp://files.example.com", false),
        Err("URL must use HTTPS (except for localhost in Local Mode)")
    );
    assert_eq!(
        validate_url("file:///etc/passwd", false),
        Err("URL must include a host")
    );
    assert_eq!(
        validate_url("ws://socket.example.com", false),
        Err("URL must use HTTPS (except for localhost in Local Mode)")
    );
}

#[test]
fn validate_url_rejects_malformed_strings() {
    assert_eq!(validate_url("not a url", false), Err("Invalid URL"));
    assert_eq!(validate_url("https://", false), Err("Invalid URL"));
}

#[test]
fn validate_url_rejects_empty_host() {
    assert_eq!(validate_url("https://?query=1", false), Err("Invalid URL"));
}

#[test]
fn validate_url_allows_empty_string() {
    assert!(validate_url("", false).is_ok());
}

#[test]
fn validate_url_allows_whitespace_only() {
    assert!(validate_url("   ", false).is_ok());
}

#[test]
fn validate_url_rejects_localhost_and_private_ips() {
    let error = Err("URL must not use a local or private host");
    assert_eq!(validate_url("https://localhost:8080", false), error);
    assert_eq!(validate_url("https://127.0.0.1/v1", false), error);
    assert_eq!(validate_url("https://0.0.0.0/v1", false), error);
    assert_eq!(validate_url("https://10.0.0.1/v1", false), error);
    assert_eq!(validate_url("https://172.16.0.1/v1", false), error);
    assert_eq!(validate_url("https://192.168.0.1/v1", false), error);
    assert_eq!(validate_url("https://169.254.0.1/v1", false), error);
    assert_eq!(validate_url("https://[::1]/v1", false), error);
    assert_eq!(validate_url("https://[::]/v1", false), error);
    assert_eq!(validate_url("https://[fc00::1]/v1", false), error);
    assert_eq!(validate_url("https://[fe80::1]/v1", false), error);
    assert_eq!(
        validate_url("https://[::ffff:192.168.0.1]/v1", false),
        error
    );
}

#[test]
fn endpoint_form_valid_rejects_invalid_current_url() {
    assert!(!is_endpoint_form_valid(
        "Endpoint",
        "http://api.example.com/v1",
        "key",
        true,
        false
    ));
}

#[test]
fn endpoint_form_valid_requires_non_empty_url() {
    assert!(!is_endpoint_form_valid("Endpoint", "", "key", true, false));
    assert!(!is_endpoint_form_valid(
        "Endpoint", "   ", "key", true, false
    ));
}

#[test]
fn endpoint_form_valid_accepts_complete_valid_form() {
    assert!(is_endpoint_form_valid(
        "Endpoint",
        "https://api.example.com/v1",
        "key",
        true,
        false
    ));
}

#[test]
fn endpoint_validation() {
    assert!(validate_url("http://localhost:11434", true).is_ok());
    assert!(validate_url("http://127.0.0.1:11434", true).is_ok());
    assert_eq!(
        validate_url("http://localhost:11434", false),
        Err("URL must use HTTPS (except for localhost in Local Mode)")
    );
    assert_eq!(
        validate_url("http://127.0.0.1:11434", false),
        Err("URL must use HTTPS (except for localhost in Local Mode)")
    );
}
