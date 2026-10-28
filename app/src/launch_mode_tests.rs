use super::agent_run_should_be_headless;

#[test]
fn top_level_gui_agent_may_open_a_window() {
    assert!(!agent_run_should_be_headless(true, None));
}

#[test]
fn orchestrated_child_stays_headless_even_when_gui_is_requested() {
    assert!(agent_run_should_be_headless(true, Some("parent-run-id")));
}

#[test]
fn blank_parent_id_does_not_turn_a_top_level_gui_run_headless() {
    assert!(!agent_run_should_be_headless(true, Some("  ")));
}
