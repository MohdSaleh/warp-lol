use warp_util::local_or_remote_path::LocalOrRemotePath;

use super::provider::NormalizedRule;

#[derive(Debug, Clone)]
pub struct ResolvedRuleSet {
    pub rules: Vec<NormalizedRule>,
}

#[derive(Debug, Clone)]
pub struct RuleResolutionTarget {
    pub working_directory: LocalOrRemotePath,
    pub target_paths: Vec<LocalOrRemotePath>,
    pub operation: GroundingOperation,
}

#[derive(Debug, Clone)]
pub enum GroundingOperation {
    Conversation,
    Read,
    Search,
    Create,
    Modify,
    Delete,
    Execute,
}

pub struct RuleResolver;

impl RuleResolver {
    pub fn resolve(
        _target: &RuleResolutionTarget,
        all_discovered: &[NormalizedRule],
    ) -> ResolvedRuleSet {
        let mut sorted = all_discovered.to_vec();

        // Sort by path depth descending (deeper first), then alphabetically
        sorted.sort_by(|a, b| {
            let a_depth = a.scope.path.display_path().split('/').count();
            let b_depth = b.scope.path.display_path().split('/').count();
            b_depth.cmp(&a_depth).then_with(|| {
                a.scope
                    .path
                    .display_path()
                    .cmp(&b.scope.path.display_path())
            })
        });

        // Wait, what defines a conflict?
        // Actually, ProjectContextModel already resolves conflicts between WARP.md and AGENTS.md in the same directory.
        // It's possible we just need to truncate if total length > 6000.
        // Let's truncate the lowest-priority (shortest depth/lexicographically later in our sort? No, sorted has highest priority first).

        let mut total_chars = 0;
        let mut rules = Vec::new();

        for mut rule in sorted {
            if total_chars >= 6000 {
                break;
            }

            let chars_left = 6000 - total_chars;
            if rule.content.len() > chars_left {
                // Truncate this rule to fit
                rule.content = rule.content.chars().take(chars_left).collect();
            }
            total_chars += rule.content.len();
            rules.push(rule);
        }

        ResolvedRuleSet { rules }
    }
}
