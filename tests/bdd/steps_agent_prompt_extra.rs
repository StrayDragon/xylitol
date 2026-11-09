use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

#[given("工具片段含 mcp 前缀工具")]
pub(crate) fn g_t2_mcp_snippets(prompt_bdd: &crate::bdd::steps_bridge::PromptBdd) {
    use xylitol_runtime::agent::prompt::{SystemPromptOpts, build_system_prompt};
    let opts = SystemPromptOpts {
        selected_tools: vec!["read".into(), "bash".into(), "mcp__fs__read".into()],
        tool_snippets: vec![
            ("read".into(), "Read file".into()),
            ("bash".into(), "Run bash".into()),
            ("mcp__fs__read".into(), "MCP read".into()),
        ],
        ..Default::default()
    };
    prompt_bdd.prompt.replace(build_system_prompt(&opts));
}

#[when("以默认路径组装系统提示")]
pub(crate) fn w_t2_build_default(prompt_bdd: &crate::bdd::steps_bridge::PromptBdd) {
    assert!(
        !prompt_bdd.prompt.borrow().is_empty(),
        "c2827: given 应已组装系统提示"
    );
}

#[then("系统提示不含 mcp 工具名且含 MCP 引导句")]
pub(crate) fn t_t2_no_mcp_names(prompt_bdd: &crate::bdd::steps_bridge::PromptBdd) {
    let p = prompt_bdd.prompt.borrow();
    assert!(
        !p.contains("mcp__fs__read"),
        "c2827: Available tools MUST NOT 枚举 mcp 名：{p}"
    );
    assert!(
        p.contains("MCP/custom tools are provided in this turn's tools list")
            && p.contains("`/mcp`"),
        "c2827: 应含 MCP 引导句：{p}"
    );
}

pub struct T2CapsBdd {
    pub caps: RefCell<Option<xylitol_runtime::agent::capabilities::AgentCapabilities>>,
}

#[fixture]
pub fn t2_caps_bdd() -> T2CapsBdd {
    T2CapsBdd {
        caps: RefCell::new(None),
    }
}

fn t2_make_caps(t2_caps_bdd: &T2CapsBdd, agent: &crate::bdd::fixtures::AgentState) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = xylitol_official_plugin_session::SessionManager::new(dir.keep());
    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(mgr);
    let caps = crate::bdd::steps_domain_compaction_extra::make_test_capabilities(agent, store);
    *t2_caps_bdd.caps.borrow_mut() = Some(caps);
}

#[given("Agent 能力聚合体已就绪")]
pub(crate) fn g_t2_caps(t2_caps_bdd: &T2CapsBdd, agent: &crate::bdd::fixtures::AgentState) {
    t2_make_caps(t2_caps_bdd, agent);
}

#[when("热应用新的上下文与系统提示资源")]
pub(crate) fn w_t2_apply_resources(t2_caps_bdd: &T2CapsBdd) {
    let mut caps = t2_caps_bdd.caps.borrow_mut().take().expect("caps");
    caps.apply_prompt_resources(
        vec![("notes.md".into(), "NOTE BODY".into())],
        Some("CUSTOM SYSTEM".into()),
        vec!["APPEND A".into()],
    );
    *t2_caps_bdd.caps.borrow_mut() = Some(caps);
}

#[then("系统提示按新资源重建")]
pub(crate) fn t_t2_rebuilt(t2_caps_bdd: &T2CapsBdd) {
    let caps = t2_caps_bdd.caps.borrow();
    let caps = caps.as_ref().expect("caps");
    let p = caps.system_prompt().expect("prompt").to_string();
    assert!(
        p.contains("CUSTOM SYSTEM") && p.contains("NOTE BODY") && p.contains("APPEND A"),
        "c2827: 热应用后系统提示应含新资源：{p}"
    );
}

#[when("热应用技能目录 greet")]
pub(crate) fn w_t2_apply_skills(t2_caps_bdd: &T2CapsBdd) {
    let mut caps = t2_caps_bdd.caps.borrow_mut().take().expect("caps");
    caps.apply_skills(vec![xylitol_plugin_sdk::resource::SkillInfo {
        name: "greet".into(),
        description: Some("greets".into()),
        source_info: xylitol_plugin_sdk::source_info::SourceInfo {
            path: std::path::PathBuf::from("/tmp/greet/SKILL.md"),
            source: "local".into(),
            scope: xylitol_plugin_sdk::source_info::SourceScope::Temporary,
            origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
            base_dir: None,
        },
        disable_model_invocation: false,
    }]);
    *t2_caps_bdd.caps.borrow_mut() = Some(caps);
}

#[then("系统提示含 available_skills 清单")]
pub(crate) fn t_t2_skills_listed(t2_caps_bdd: &T2CapsBdd) {
    let caps = t2_caps_bdd.caps.borrow();
    let caps = caps.as_ref().expect("caps");
    assert!(
        caps.loaded_skill_names().contains(&"greet".to_string()),
        "c2827: 技能目录应进入注册表"
    );
    let p = caps.system_prompt().expect("prompt").to_string();
    assert!(
        p.contains("greet"),
        "c2827: 系统提示应含 available_skills 条目 greet：{p}"
    );
}

pub struct T2SkillBdd {
    pub skills: RefCell<Vec<xylitol_plugin_sdk::resource::SkillInfo>>,
    pub expanded: RefCell<String>,
}

#[fixture]
pub fn t2_skill_bdd() -> T2SkillBdd {
    T2SkillBdd {
        skills: RefCell::new(Vec::new()),
        expanded: RefCell::new(String::new()),
    }
}

#[given("已加载技能 greet 正文为 {body:string}")]
pub(crate) fn g_t2_skill_loaded(t2_skill_bdd: &T2SkillBdd, body: String) {
    let body = body.trim_matches('"').to_string();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SKILL.md");
    std::fs::write(
        &path,
        format!("---\nname: greet\ndescription: greets\n---\n{body}"),
    )
    .expect("write skill");
    std::mem::forget(dir);
    *t2_skill_bdd.skills.borrow_mut() = vec![xylitol_plugin_sdk::resource::SkillInfo {
        name: "greet".into(),
        description: None,
        source_info: xylitol_plugin_sdk::source_info::SourceInfo {
            path,
            source: "local".into(),
            scope: xylitol_plugin_sdk::source_info::SourceScope::Temporary,
            origin: xylitol_plugin_sdk::source_info::SourceOrigin::TopLevel,
            base_dir: None,
        },
        disable_model_invocation: false,
    }];
}

#[when("对含技能引用的输入展开技能")]
pub(crate) fn w_t2_expand(t2_skill_bdd: &T2SkillBdd) {
    let skills = t2_skill_bdd.skills.borrow().clone();
    let out = xylitol_runtime::agent::prompt::skill_expand::expand_skill_refs(
        "请执行 $greet 与 $nope",
        &skills,
    );
    *t2_skill_bdd.expanded.borrow_mut() = out;
}

#[then("投影注入技能正文且未知名透传")]
pub(crate) fn t_t2_expanded(t2_skill_bdd: &T2SkillBdd) {
    let out = t2_skill_bdd.expanded.borrow().clone();
    assert!(
        out.contains("SKILL BODY"),
        "c2827: $greet 应注入 SKILL.md 正文：{out}"
    );
    assert!(out.contains("$nope"), "c2827: 未知名 MUST 透传：{out}");
}
