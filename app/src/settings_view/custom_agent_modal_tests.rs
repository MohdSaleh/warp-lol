use pathfinder_geometry::vector::vec2f;
use warpui::platform::WindowStyle;
use warpui::scene::Scene;
use warpui::ui_components::components::{Coords, UiComponentStyles};
use warpui::{App, EntityIdSet, Presenter, WindowInvalidation};

use super::*;
use crate::test_util::terminal::initialize_app_for_terminal_view;

fn modal_height(scene: &Scene) -> f32 {
    scene
        .layers()
        .flat_map(|layer| &layer.rects)
        .filter(|rect| rect.bounds.width() > 500. && rect.bounds.width() <= 560.)
        .map(|rect| rect.bounds.height())
        .max_by(f32::total_cmp)
        .expect("custom agent modal rect should exist")
}

#[test]
fn long_instructions_keep_the_editor_and_modal_height_bounded() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (window_id, modal) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let body = ctx.add_typed_action_view(CustomAgentModal::new);
            Modal::new(Some("Create Agent".to_string()), body, ctx)
                .with_modal_style(UiComponentStyles {
                    width: Some(560.),
                    ..Default::default()
                })
                .with_body_style(UiComponentStyles {
                    padding: Some(Coords::uniform(24.)),
                    ..Default::default()
                })
        });
        let body = modal.read(&app, |modal, _| modal.body().clone());
        let instructions_editor = body.read(&app, |body, _| body.instructions_editor.clone());
        let invalidation = WindowInvalidation {
            updated: EntityIdSet::from_iter([
                app.root_view_id(window_id).expect("root view should exist"),
                body.id(),
                instructions_editor.id(),
            ]),
            ..Default::default()
        };
        let mut presenter = Presenter::new(window_id);

        app.update(move |ctx| {
            presenter.invalidate(invalidation.clone(), ctx);
            let initial_height =
                modal_height(&presenter.build_scene(vec2f(800., 800.), 1., None, ctx));

            instructions_editor.update(ctx, |editor, ctx| {
                editor.set_buffer_text(&"Long instruction line\n".repeat(200), ctx);
            });
            presenter.invalidate(invalidation, ctx);
            let long_content_height =
                modal_height(&presenter.build_scene(vec2f(800., 800.), 1., None, ctx));

            assert_eq!(
                initial_height, long_content_height,
                "long instructions must scroll inside the fixed-height editor"
            );
            assert_eq!(INSTRUCTIONS_INPUT_HEIGHT, 144.);
        });
    })
}
