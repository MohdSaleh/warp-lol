#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPriority {
    Critical,
    High,
    Medium,
    Low,
}

pub struct ToolSelectionConfig {
    pub allow_optional_tools: bool,
    pub query_context: Option<String>,
}

pub fn select_tools(available_tools: &[String], config: &ToolSelectionConfig) -> Vec<String> {
    let core_tools = [
        "run_command",
        "read_files",
        "apply_file_diffs",
        "grep",
        "glob",
        "update_todos",
        "ask_user_question",
    ];

    let mut selected = Vec::new();
    for tool in available_tools {
        let is_core = core_tools.contains(&tool.as_str());
        if is_core || config.allow_optional_tools {
            selected.push(tool.clone());
        }
    }
    selected
}
