use rstest_bdd_macros::then;

#[then("登录引导含 /login 与文档路径")]
pub(crate) fn t_t4_login_help() {
    let msg = xylitol::cli::get_provider_login_help();
    assert!(
        msg.contains("/login") && (msg.contains("providers.md") || msg.contains("docs")),
        "c2827: 登录引导应引用 /login 与文档：{msg}"
    );
}

#[then("无可用模型提示衔接登录引导")]
pub(crate) fn t_t4_no_models() {
    let msg = xylitol::cli::format_no_models_available_message();
    assert!(
        msg.contains("No models available") && msg.contains("/login"),
        "c2827: 无模型提示应存在并引导登录：{msg}"
    );
}

#[then("未选模型展示占位而非厂商默认名")]
pub(crate) fn t_t4_unset_model() {
    assert_eq!(
        xylitol_kernel::bootstrap::UNSET_MODEL_DISPLAY,
        "NOT-SET",
        "c2827: 未选模型占位 MUST NOT 冒充厂商默认模型名"
    );
}

#[then("无 api key 提示含 provider 名")]
pub(crate) fn t_t4_no_api_key() {
    let msg = xylitol::cli::format_no_api_key_found_message("openai");
    assert!(
        msg.contains("openai"),
        "c2827: 无 key 提示应含 provider 名：{msg}"
    );
}
