use warp_multi_agent_api as api;

use super::*;

fn params_with_model() -> RequestParams {
    let mut params = RequestParams::new_for_test();
    params.model = "cfg-1".into();
    params.custom_model_providers = Some(api::request::settings::CustomModelProviders {
        providers: vec![
            api::request::settings::custom_model_providers::CustomModelProvider {
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                models: vec![
                    api::request::settings::custom_model_providers::CustomModel {
                        slug: "real-model".to_string(),
                        config_key: "cfg-1".to_string(),
                    },
                ],
            },
        ],
    });
    params
}

#[test]
fn selects_custom_model_by_config_key() {
    let model = select_model(&params_with_model()).unwrap();
    assert_eq!(model.slug, "real-model");
    assert_eq!(model.base_url, "https://api.example.com/v1");
}

#[test]
fn select_model_requires_custom_provider() {
    assert!(select_model(&RequestParams::new_for_test()).is_err());
}

#[test]
fn chat_url_normalizes() {
    assert_eq!(
        chat_completions_url("https://api.example.com/v1").unwrap(),
        "https://api.example.com/v1/chat/completions"
    );
    assert_eq!(
        chat_completions_url("https://api.example.com/v1/models").unwrap(),
        "https://api.example.com/v1/chat/completions"
    );
}
