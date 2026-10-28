use serde_json::Value;
use warp_multi_agent_api::ask_user_question::question::QuestionType;
use warp_multi_agent_api::ask_user_question::{MultipleChoice, Option as QuestionOption, Question};
use warp_multi_agent_api::ask_user_question_result::answer_item::{Answer, MultipleChoiceAnswer};
use warp_multi_agent_api::ask_user_question_result::{
    AnswerItem, Result as QuestionResult, Success,
};
use warp_multi_agent_api::message::tool_call as tc;
use warp_multi_agent_api::message::tool_call_result::Result as ToolCallResultType;

use super::*;

#[test]
fn ask_user_question_history_preserves_question_and_options() {
    let tool = tc::Tool::AskUserQuestion(warp_multi_agent_api::AskUserQuestion {
        questions: vec![Question {
            question_id: "q0".to_string(),
            question: "Which database?".to_string(),
            question_type: Some(QuestionType::MultipleChoice(MultipleChoice {
                options: vec![
                    QuestionOption {
                        label: "SQLite".to_string(),
                    },
                    QuestionOption {
                        label: "PostgreSQL".to_string(),
                    },
                ],
                recommended_option_index: 0,
                is_multiselect: false,
                supports_other: true,
            })),
        }],
    });

    let (name, arguments) = history_ask_user_question(&tool).unwrap();
    let arguments: Value = serde_json::from_str(&arguments).unwrap();

    assert_eq!(name, "ask_user_question");
    assert_eq!(arguments["questions"][0]["question_id"], "q0");
    assert_eq!(arguments["questions"][0]["question"], "Which database?");
    assert_eq!(arguments["questions"][0]["options"][1], "PostgreSQL");
    assert_eq!(arguments["questions"][0]["supports_other"], true);
}

#[test]
fn ask_user_question_result_preserves_selected_option_for_the_model() {
    let result = warp_multi_agent_api::message::ToolCallResult {
        tool_call_id: "question-call".to_string(),
        context: None,
        result: Some(ToolCallResultType::AskUserQuestion(
            warp_multi_agent_api::AskUserQuestionResult {
                result: Some(QuestionResult::Success(Success {
                    answers: vec![
                        AnswerItem {
                            question_id: "q0".to_string(),
                            answer: Some(Answer::MultipleChoice(MultipleChoiceAnswer {
                                selected_options: vec!["PostgreSQL".to_string()],
                                other_text: "Use the managed cluster".to_string(),
                            })),
                        },
                        AnswerItem {
                            question_id: "q1".to_string(),
                            answer: Some(Answer::Skipped(())),
                        },
                    ],
                })),
            },
        )),
    };

    let envelope = render_ask_user_question_result(&result).unwrap();
    let data = envelope.data.unwrap();

    assert_eq!(data["answers"][0]["question_id"], "q0");
    assert_eq!(data["answers"][0]["selected_options"][0], "PostgreSQL");
    assert_eq!(data["answers"][0]["other_text"], "Use the managed cluster");
    assert_eq!(data["answers"][0]["skipped"], false);
    assert_eq!(data["answers"][1]["question_id"], "q1");
    assert_eq!(data["answers"][1]["skipped"], true);
}
