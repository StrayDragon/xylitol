use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};

#[then("列表含 skills 与 themes 且不含 prompts")]
pub(crate) fn t_t2_res_list_no_prompts(prompt_bdd: &crate::bdd::steps_bridge::PromptBdd) {
    let out = prompt_bdd.prompt.borrow().clone();
    assert!(
        out.contains("skills:") && out.contains("themes:"),
        "c2827: 资源列表应含 skills 与 themes：{out}"
    );
    assert!(
        !out.contains("prompts:") && !out.contains("greet"),
        "c2827: prompts/*.md MUST NOT 列为资源：{out}"
    );
}

pub struct T2ResBdd {
    pub cwd: RefCell<Option<std::path::PathBuf>>,
    pub agent_dir: RefCell<Option<std::path::PathBuf>>,
    pub out: RefCell<String>,
}

#[fixture]
pub fn t2_res_bdd() -> T2ResBdd {
    T2ResBdd {
        cwd: RefCell::new(None),
        agent_dir: RefCell::new(None),
        out: RefCell::new(String::new()),
    }
}

fn t2_res_mount(t2_res_bdd: &T2ResBdd) {
    if t2_res_bdd.cwd.borrow().is_some() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().join("project");
    let agent_dir = tmp.path().join("home").join(".xylitol");
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&agent_dir).unwrap();
    std::fs::create_dir_all(agent_dir.join("skills").join("demo-skill")).unwrap();
    std::fs::write(
        agent_dir.join("skills").join("demo-skill").join("SKILL.md"),
        "---\nname: demo-skill\ndescription: demo\n---\nbody",
    )
    .unwrap();
    std::fs::create_dir_all(agent_dir.join("themes")).unwrap();
    std::fs::write(agent_dir.join("themes").join("dark.json"), "{}").unwrap();
    *t2_res_bdd.cwd.borrow_mut() = Some(cwd);
    *t2_res_bdd.agent_dir.borrow_mut() = Some(agent_dir);
    std::mem::forget(tmp);
}

#[given("资源目录含 demo 技能与 dark 主题")]
pub(crate) fn g_t2_res_layout(t2_res_bdd: &T2ResBdd) {
    t2_res_mount(t2_res_bdd);
}

#[when("经 loader 列出已发现资源")]
pub(crate) fn w_t2_res_list(t2_res_bdd: &T2ResBdd) {
    use xylitol_runtime::resource::DefaultResourceLoader;

    let cwd = t2_res_bdd.cwd.borrow().as_ref().expect("cwd").clone();
    let agent_dir = t2_res_bdd
        .agent_dir
        .borrow()
        .as_ref()
        .expect("agent")
        .clone();
    let loader = DefaultResourceLoader::new(cwd, agent_dir);
    let (skills, _) = loader.get_skills();
    let (themes, _) = loader.get_themes();
    let mut out = String::from("skills:\n");
    for s in skills {
        out.push_str(&format!("  {}\n", s.name));
    }
    out.push_str("themes:\n");
    for t in themes {
        out.push_str(&format!("  {}\n", t.name));
    }
    *t2_res_bdd.out.borrow_mut() = out;
}

#[then("列出技能名与主题 stem")]
pub(crate) fn t_t2_res_list_ok(t2_res_bdd: &T2ResBdd) {
    let out = t2_res_bdd.out.borrow().clone();
    assert!(
        out.contains("skills:") && out.contains("demo-skill"),
        "loader 应含技能：{out}"
    );
    assert!(
        out.contains("themes:") && out.contains("dark"),
        "loader 应列出 theme stem：{out}"
    );
}

#[when("以压后上下文检查 bash 排除")]
pub(crate) fn w_t2_bash_exclude() {}

#[then("排除的 bash 条目不进上下文而普通条目保留")]
pub(crate) fn t_t2_bash_exclude_filtered() {
    use xylitol_plugin_sdk::message::BashExecutionStatus;
    use xylitol_plugin_sdk::session::build_context_entries;
    let exclude = xylitol_plugin_sdk::session::bash_execution_message_entry(
        "b1",
        "make test",
        "all ok",
        Some(0),
        false,
        false,
        None,
        true,
        BashExecutionStatus::Done,
    );
    let keep = xylitol_plugin_sdk::session::bash_execution_message_entry(
        "b2",
        "make build",
        "ok",
        Some(0),
        false,
        false,
        None,
        false,
        BashExecutionStatus::Done,
    );
    let entries = vec![exclude, keep];
    let ctx = build_context_entries(&entries);
    let rendered: Vec<String> = ctx
        .iter()
        .filter_map(|e| e.as_agent_message().map(|m| format!("{m:?}")))
        .collect();
    let joined = rendered.join("\n");
    assert!(
        !joined.contains("make test"),
        "c2827: exclude 条目 MUST NOT 进上下文：{joined}"
    );
    assert!(
        joined.contains("make build"),
        "c2827: 非 exclude 条目应保留：{joined}"
    );
}
