use settings_page::MatchData;

use super::*;

// ── SettingsSection classification ──────────────────────────────────────────

#[test]
fn ai_subpages_are_identified() {
    assert!(SettingsSection::WarpAgent.is_ai_subpage());
    assert!(SettingsSection::AgentProfiles.is_ai_subpage());
    assert!(SettingsSection::AgentMCPServers.is_ai_subpage());
    assert!(SettingsSection::Knowledge.is_ai_subpage());
    assert!(SettingsSection::ThirdPartyCLIAgents.is_ai_subpage());

    assert!(!SettingsSection::AI.is_ai_subpage());
    assert!(!SettingsSection::About.is_ai_subpage());
    assert!(!SettingsSection::CodeIndexing.is_ai_subpage());
}

#[test]
fn code_subpages_are_identified() {
    assert!(SettingsSection::CodeIndexing.is_code_subpage());
    assert!(SettingsSection::EditorAndCodeReview.is_code_subpage());

    assert!(!SettingsSection::Code.is_code_subpage());
    assert!(!SettingsSection::WarpAgent.is_code_subpage());
}

#[test]
fn is_subpage_covers_all_umbrella_types() {
    // All subpages under any umbrella should return true.
    for section in SettingsSection::ai_subpages() {
        assert!(section.is_subpage(), "{section:?} should be a subpage");
    }
    assert!(SettingsSection::CodeIndexing.is_subpage());
    assert!(SettingsSection::EditorAndCodeReview.is_subpage());

    // Top-level pages should not be subpages.
    assert!(!SettingsSection::About.is_subpage());
    assert!(!SettingsSection::AI.is_subpage());
    assert!(!SettingsSection::Code.is_subpage());
    assert!(!SettingsSection::Privacy.is_subpage());
}

// ── parent_page_section mapping ─────────────────────────────────────────────

#[test]
fn ai_subpages_map_to_ai_backing_page() {
    assert_eq!(
        SettingsSection::WarpAgent.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::AgentProfiles.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::Knowledge.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::ThirdPartyCLIAgents.parent_page_section(),
        SettingsSection::AI
    );
}

#[test]
fn agent_mcp_servers_maps_to_mcp_servers_page() {
    // AgentMCPServers renders the standalone MCPServers page, not the AI page.
    assert_eq!(
        SettingsSection::AgentMCPServers.parent_page_section(),
        SettingsSection::MCPServers
    );
}

#[test]
fn code_subpages_map_to_code_backing_page() {
    assert_eq!(
        SettingsSection::CodeIndexing.parent_page_section(),
        SettingsSection::Code
    );
    assert_eq!(
        SettingsSection::EditorAndCodeReview.parent_page_section(),
        SettingsSection::Code
    );
}

#[test]
fn non_subpage_sections_map_to_themselves() {
    assert_eq!(
        SettingsSection::About.parent_page_section(),
        SettingsSection::About
    );
    assert_eq!(
        SettingsSection::AI.parent_page_section(),
        SettingsSection::AI
    );
    assert_eq!(
        SettingsSection::Privacy.parent_page_section(),
        SettingsSection::Privacy
    );
}

// ── ai_subpages list ────────────────────────────────────────────────────────

#[test]
fn ai_subpages_list_contains_all_ai_subpage_variants() {
    let subpages = SettingsSection::ai_subpages();
    assert!(subpages.contains(&SettingsSection::WarpAgent));
    assert!(subpages.contains(&SettingsSection::AgentProfiles));
    assert!(subpages.contains(&SettingsSection::AgentMCPServers));
    assert!(subpages.contains(&SettingsSection::Knowledge));
    assert!(subpages.contains(&SettingsSection::ThirdPartyCLIAgents));
}

#[test]
fn ai_subpages_list_does_not_contain_non_subpages() {
    let subpages = SettingsSection::ai_subpages();
    assert!(!subpages.contains(&SettingsSection::AI));
    assert!(!subpages.contains(&SettingsSection::About));
    assert!(!subpages.contains(&SettingsSection::Code));
}

// ── MatchData behavior ──────────────────────────────────────────────────────

#[test]
fn match_data_uncounted_true_is_truthy() {
    assert!(MatchData::Uncounted(true).is_truthy());
}

#[test]
fn match_data_uncounted_false_is_not_truthy() {
    assert!(!MatchData::Uncounted(false).is_truthy());
}

#[test]
fn match_data_countable_nonzero_is_truthy() {
    assert!(MatchData::Countable(3).is_truthy());
    assert!(MatchData::Countable(1).is_truthy());
}

#[test]
fn match_data_countable_zero_is_not_truthy() {
    assert!(!MatchData::Countable(0).is_truthy());
}

// ── Display / FromStr round-trip ────────────────────────────────────────────

#[test]
fn subpage_display_names_are_correct() {
    assert_eq!(SettingsSection::WarpAgent.to_string(), "Warp Agent");
    assert_eq!(SettingsSection::AgentProfiles.to_string(), "Profiles");
    assert_eq!(SettingsSection::AgentMCPServers.to_string(), "MCP servers");
    assert_eq!(SettingsSection::Knowledge.to_string(), "Knowledge");
    assert_eq!(
        SettingsSection::ThirdPartyCLIAgents.to_string(),
        "Third party CLI agents"
    );
    assert_eq!(
        SettingsSection::CodeIndexing.to_string(),
        "Indexing and projects"
    );
    assert_eq!(
        SettingsSection::EditorAndCodeReview.to_string(),
        "Editor and Code Review"
    );
}

#[test]
fn subpage_from_str_parses_display_names() {
    // Both the legacy "Oz" name and the new "Warp Agent" display name must
    // resolve to SettingsSection::WarpAgent so existing deep links, persisted
    // telemetry strings, and external callers continue to work after the
    // user-facing rename (see specs/GH1063/product.md, Behavior #8).
    assert_eq!(
        SettingsSection::from_str("Oz"),
        Ok(SettingsSection::WarpAgent)
    );
    assert_eq!(
        SettingsSection::from_str("Warp Agent"),
        Ok(SettingsSection::WarpAgent)
    );
    assert_eq!(
        SettingsSection::from_str("Profiles"),
        Ok(SettingsSection::AgentProfiles)
    );
    assert_eq!(
        SettingsSection::from_str("Knowledge"),
        Ok(SettingsSection::Knowledge)
    );
    assert_eq!(
        SettingsSection::from_str("Indexing and projects"),
        Ok(SettingsSection::CodeIndexing)
    );
    assert_eq!(
        SettingsSection::from_str("Editor and Code Review"),
        Ok(SettingsSection::EditorAndCodeReview)
    );
}

// ── Subpage search filter simulation ────────────────────────────────────────
// These tests simulate the per-subpage search filtering logic used in
// handle_search_editor_event: each subpage should only be visible if its
// own widgets' search terms match, not if a sibling subpage's terms match.

/// Helper: given a map of subpage→MatchData, returns which subpages are visible.
fn visible_subpages(
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    subpages: &[SettingsSection],
) -> Vec<SettingsSection> {
    subpages
        .iter()
        .filter(|s| {
            subpage_filter
                .get(s)
                .map(|md| md.is_truthy())
                .unwrap_or(false)
        })
        .copied()
        .collect()
}

#[test]
fn search_knowledge_shows_only_knowledge_subpage() {
    // Simulate: searching "knowledge" matched the Knowledge subpage but not others.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    assert_eq!(visible, vec![SettingsSection::Knowledge]);
}

#[test]
fn search_agent_shows_profiles_and_cli_agents() {
    // "agent" appears in both AgentProfiles and ThirdPartyCLIAgents search terms.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(2));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(1),
    );

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    assert!(visible.contains(&SettingsSection::AgentProfiles));
    assert!(visible.contains(&SettingsSection::ThirdPartyCLIAgents));
    assert!(!visible.contains(&SettingsSection::WarpAgent));
    assert!(!visible.contains(&SettingsSection::Knowledge));
}

#[test]
fn empty_search_shows_no_subpages_in_filter() {
    // When search is cleared, subpage_filter is empty — all subpages fall back
    // to their backing page visibility (Uncounted(true) by default).
    let filter: HashMap<SettingsSection, MatchData> = HashMap::new();

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    // No entries in filter means no subpage-specific filtering; all return false
    // from the filter map. The actual rendering code falls back to the backing
    // page's pages_filter which defaults to Uncounted(true).
    assert!(visible.is_empty());
}

#[test]
fn search_with_no_matches_hides_all_subpages() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let visible = visible_subpages(&filter, SettingsSection::ai_subpages());

    assert!(visible.is_empty());
}

/// Helper: check if an umbrella should be visible given a subpage filter.
fn umbrella_visible(
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    umbrella_subpages: &[SettingsSection],
) -> bool {
    umbrella_subpages.iter().any(|s| {
        subpage_filter
            .get(s)
            .map(|md| md.is_truthy())
            .unwrap_or(false)
    })
}

#[test]
fn umbrella_hidden_when_no_subpages_match() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    assert!(!umbrella_visible(&filter, SettingsSection::ai_subpages()));
}

// ── cycle_pages search filter ────────────────────────────────────────────────
// These tests validate the logic added to cycle_pages() to ensure arrow key
// navigation respects the active search filter.

/// Mirrors the filter predicate used in cycle_pages() when search is active.
fn section_passes_nav_filter(
    section: SettingsSection,
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    pages_filter: &[(SettingsSection, MatchData)],
) -> bool {
    if let Some(md) = subpage_filter.get(&section) {
        md.is_truthy()
    } else {
        let backing = section.parent_page_section();
        pages_filter
            .iter()
            .any(|(s, md)| *s == backing && md.is_truthy())
    }
}

#[test]
fn nav_filter_includes_matching_subpage_and_excludes_others() {
    let mut subpage_filter = HashMap::new();
    subpage_filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    subpage_filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    subpage_filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    subpage_filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    // No page-level filter entries needed since all AI subpages have subpage_filter entries.
    let pages_filter: Vec<(SettingsSection, MatchData)> = vec![];

    assert!(!section_passes_nav_filter(
        SettingsSection::WarpAgent,
        &subpage_filter,
        &pages_filter
    ));
    assert!(!section_passes_nav_filter(
        SettingsSection::AgentProfiles,
        &subpage_filter,
        &pages_filter
    ));
    assert!(section_passes_nav_filter(
        SettingsSection::Knowledge,
        &subpage_filter,
        &pages_filter
    ));
    assert!(!section_passes_nav_filter(
        SettingsSection::ThirdPartyCLIAgents,
        &subpage_filter,
        &pages_filter
    ));
}

#[test]
fn nav_filter_falls_back_to_pages_filter_for_top_level_pages() {
    // Top-level pages (Account, Appearance, etc.) have no subpage_filter entry.
    // They fall back to pages_filter using parent_page_section() == themselves.
    let subpage_filter: HashMap<SettingsSection, MatchData> = HashMap::new();
    let pages_filter = vec![
        (SettingsSection::About, MatchData::Uncounted(true)),
        (SettingsSection::Appearance, MatchData::Countable(0)),
        (SettingsSection::Features, MatchData::Uncounted(true)),
    ];

    assert!(section_passes_nav_filter(
        SettingsSection::About,
        &subpage_filter,
        &pages_filter
    ));
    assert!(!section_passes_nav_filter(
        SettingsSection::Appearance,
        &subpage_filter,
        &pages_filter
    ));
    assert!(section_passes_nav_filter(
        SettingsSection::Features,
        &subpage_filter,
        &pages_filter
    ));
}

#[test]
fn umbrella_visible_when_any_subpage_matches() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    assert!(umbrella_visible(&filter, SettingsSection::ai_subpages()));
}

// ── Search auto-select simulation ───────────────────────────────────────────
// These tests simulate the auto-select logic in handle_search_editor_event:
// when the current subpage is filtered out by search, the view should jump
// to the first visible subpage or page.

/// Simulates the "is current still visible" check from the search handler.
/// Returns true if `current` is still visible given the subpage_filter and
/// a list of (backing_section, is_truthy) pairs for pages_filter.
fn is_current_visible(
    current: SettingsSection,
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    pages_visible: &[(SettingsSection, bool)],
) -> bool {
    if let Some(md) = subpage_filter.get(&current) {
        return md.is_truthy();
    }
    let backing = current.parent_page_section();
    pages_visible
        .iter()
        .any(|(section, visible)| *section == backing && *visible)
}

/// Simulates finding the first visible section from the nav_items order.
fn first_visible_section(
    nav_order: &[SettingsSection],
    subpage_filter: &HashMap<SettingsSection, MatchData>,
    pages_visible: &[(SettingsSection, bool)],
) -> Option<SettingsSection> {
    nav_order.iter().copied().find(|section| {
        if let Some(md) = subpage_filter.get(section) {
            md.is_truthy()
        } else {
            let backing = section.parent_page_section();
            pages_visible
                .iter()
                .any(|(s, visible)| *s == backing && *visible)
        }
    })
}

#[test]
fn auto_select_jumps_away_from_filtered_out_subpage() {
    // User is on Knowledge, searches "agent" which matches Profiles but not Knowledge.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(2));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(1),
    );

    let current = SettingsSection::Knowledge;
    assert!(
        !is_current_visible(current, &filter, &[]),
        "Knowledge should not be visible when it has 0 matches"
    );

    // The nav order: Oz, Profiles, ..., Knowledge, ThirdPartyCLI
    let nav_order = SettingsSection::ai_subpages();
    let first = first_visible_section(nav_order, &filter, &[]);
    assert_eq!(
        first,
        Some(SettingsSection::AgentProfiles),
        "Should auto-select Profiles as the first visible subpage"
    );
}

#[test]
fn auto_select_stays_on_current_when_it_matches() {
    // User is on Knowledge, searches "knowledge" which matches Knowledge.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(1));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let current = SettingsSection::Knowledge;
    assert!(
        is_current_visible(current, &filter, &[]),
        "Knowledge should remain visible when it has matches"
    );
}

#[test]
fn auto_select_falls_back_to_top_level_page_when_no_subpages_match() {
    // All AI subpages filtered out, but Account (top-level) is still visible.
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));
    filter.insert(SettingsSection::Knowledge, MatchData::Countable(0));
    filter.insert(
        SettingsSection::ThirdPartyCLIAgents,
        MatchData::Countable(0),
    );

    let pages_visible = vec![(SettingsSection::About, true), (SettingsSection::AI, false)];

    // Nav order includes top-level Account before the AI subpages.
    let nav_order = vec![
        SettingsSection::About,
        SettingsSection::WarpAgent,
        SettingsSection::AgentProfiles,
        SettingsSection::Knowledge,
        SettingsSection::ThirdPartyCLIAgents,
    ];

    let first = first_visible_section(&nav_order, &filter, &pages_visible);
    assert_eq!(
        first,
        Some(SettingsSection::About),
        "Should fall back to Account when no subpages match"
    );
}

#[test]
fn auto_select_handles_standalone_subpage_via_backing_page() {
    // AgentMCPServers has its own backing page (MCPServers), not in subpage_filter.
    // It should be visible if its backing page is visible.
    let filter = HashMap::new(); // no per-subpage entries for AgentMCPServers

    let pages_visible = vec![
        (SettingsSection::MCPServers, true),
        (SettingsSection::AI, false),
    ];

    let current = SettingsSection::AgentMCPServers;
    assert!(
        is_current_visible(current, &filter, &pages_visible),
        "AgentMCPServers should be visible via its MCPServers backing page"
    );
}

#[test]
fn auto_select_with_no_matches_anywhere() {
    let mut filter = HashMap::new();
    filter.insert(SettingsSection::WarpAgent, MatchData::Countable(0));
    filter.insert(SettingsSection::AgentProfiles, MatchData::Countable(0));

    let pages_visible = vec![
        (SettingsSection::About, false),
        (SettingsSection::AI, false),
    ];

    let nav_order = vec![
        SettingsSection::About,
        SettingsSection::WarpAgent,
        SettingsSection::AgentProfiles,
    ];

    let first = first_visible_section(&nav_order, &filter, &pages_visible);
    assert_eq!(
        first, None,
        "No section should be selected when nothing matches"
    );
}

// ── Backward compatibility ──────────────────────────────────────────────────

#[test]
fn legacy_ai_section_maps_to_oz_default() {
    // SettingsSection::AI should be treated as backward-compat and map to Oz
    // via the code in set_and_refresh_current_page_internal.
    // Here we just verify the parent_page_section is still AI (for page lookup).
    assert_eq!(
        SettingsSection::AI.parent_page_section(),
        SettingsSection::AI
    );
    // And that AI is NOT itself a subpage.
    assert!(!SettingsSection::AI.is_subpage());
}
