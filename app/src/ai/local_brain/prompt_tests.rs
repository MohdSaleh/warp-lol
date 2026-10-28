use std::path::PathBuf;

use ai::skills::{SkillProvider, SkillReference, SkillScope};

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentContext, AIAgentInput};
use crate::ai::skills::SkillDescriptor;

#[test]
fn test_skills_injected_into_prompt() {
    let mut params = RequestParams::new_for_test();

    // Add skills context
    let skills = vec![SkillDescriptor {
        name: "test-skill".to_string(),
        description: "A test skill".to_string(),
        reference: SkillReference::Path(warp_util::local_or_remote_path::LocalOrRemotePath::Local(
            PathBuf::from("/path/to/skill"),
        )),
        scope: SkillScope::Project,
        provider: SkillProvider::Warp,
        icon_override: None,
    }];

    params.input.push(AIAgentInput::UserQuery {
        query: "help me".to_string(),
        context: vec![AIAgentContext::Skills { skills }].into(),
        static_query_type: Default::default(),
        referenced_attachments: Default::default(),
        user_query_mode: Default::default(),
        running_command: None,
        intended_agent: None,
    });

    let prompt = crate::ai::local_brain::prompt::system_prompt(&params);
    assert!(prompt.contains("Available Skills"));
    assert!(prompt.contains("1. test-skill: A test skill"));
}

#[test]
fn test_no_skills_no_section() {
    let params = RequestParams::new_for_test();
    let prompt = crate::ai::local_brain::prompt::system_prompt(&params);
    assert!(!prompt.contains("Available Skills"));
}

#[test]
fn test_planning_uses_structured_todos_instead_of_text_checklists() {
    let mut params = RequestParams::new_for_test();
    params.planning_enabled = true;

    let prompt = crate::ai::local_brain::prompt::system_prompt(&params);

    assert!(prompt.contains("call `update_todos` (op `create`)"));
    assert!(prompt.contains("Never print the checklist as chat text"));
    assert!(prompt.contains("pair it with exactly one execution tool call"));
    assert!(!prompt.contains("Immediately output a clear, complete, numbered checklist"));
}
