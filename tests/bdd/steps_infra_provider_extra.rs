use crate::bdd::steps_agent_tools_extra::T2SchemaBdd;
use rstest_bdd_macros::{given, then};

#[given("Responses 组装且消息含 user 与 assistant")]
pub(crate) fn g_t2_input_items(t2_schema_bdd: &T2SchemaBdd) {
    use xylitol_official_plugin_provider_openai::dto::XyLlmMessage;
    use xylitol_official_plugin_provider_openai::provider::messages_to_responses_input_with_options;
    use xylitol_official_plugin_provider_openai::thinking::XyGenerateOptions;
    let msgs = vec![
        XyLlmMessage::user("问一下"),
        XyLlmMessage::assistant("答一句"),
    ];
    let input = messages_to_responses_input_with_options(&msgs, &XyGenerateOptions::default());
    let v = serde_json::to_value(&input).unwrap();
    *t2_schema_bdd.schemas.borrow_mut() = vec![("input".into(), v)];
}

#[then("每个 input item 均带 type 字段")]
pub(crate) fn t_t2_input_items_typed(t2_schema_bdd: &T2SchemaBdd) {
    let schemas = t2_schema_bdd.schemas.borrow();
    let v = &schemas[0].1;
    let items = v.as_array().expect("input items array");
    assert!(!items.is_empty(), "c2827: input 不应为空");
    for item in items {
        assert!(
            item.get("type").and_then(|t| t.as_str()).is_some(),
            "c2827: input item MUST 带 type：{item}"
        );
    }
}
