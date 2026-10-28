use super::provider::NormalizedRule;

pub struct RuleRenderer;

impl RuleRenderer {
    pub fn render_for_prompt(rules: &[NormalizedRule]) -> String {
        if rules.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        out.push_str("=== PROJECT RULES ===\n");
        for rule in rules {
            out.push_str(&format!("# From {}\n", rule.scope.path.display_path()));
            out.push_str(&rule.content);
            out.push('\n');
        }
        out.push_str("=====================\n");
        out
    }
}
