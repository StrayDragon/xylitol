use crate::bdd::fixtures::AgentState;
use crate::bdd::helpers::make_agent_with_store;
use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol::embed::{XyDriver, XyInProcessDriver, assemble_driver};
use xylitol_plugin_sdk::session::CapabilitySnapshot;

pub struct CapabilityBdd {
    pub driver: RefCell<Option<XyInProcessDriver>>,
    pub snapshot: RefCell<Option<CapabilitySnapshot>>,
}

#[fixture]
pub fn capability_bdd() -> CapabilityBdd {
    CapabilityBdd {
        driver: RefCell::new(None),
        snapshot: RefCell::new(None),
    }
}

#[given("会话已装配工具 {tool:string} 且工具表已冻结")]
pub(crate) async fn g_cap_frozen(agent: &AgentState, capability_bdd: &CapabilityBdd, tool: String) {
    let tool = tool.trim_matches('"');
    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = assemble_driver(runtime, store);
    driver.ensure_tool_table_frozen().await;
    assert!(
        driver.tool_names_for_test().iter().any(|name| name == tool),
        "capability: base tool `{tool}` 未装配"
    );
    *capability_bdd.driver.borrow_mut() = Some(driver);
}

#[when("禁用能力 {id:string} 并开始下一回合")]
pub(crate) async fn w_cap_disable_next_turn(capability_bdd: &CapabilityBdd, id: String) {
    let id = id.trim_matches('"');
    let mut driver = capability_bdd
        .driver
        .borrow_mut()
        .take()
        .expect("capability driver mounted");
    driver.set_capability(id, false).expect("set_capability");
    assert!(
        driver.capability_pending(),
        "capability: 禁用后须处于待生效状态"
    );
    let mut stream = driver.run("capability-overlay").await;
    while stream.next().await.is_some() {}
    *capability_bdd.driver.borrow_mut() = Some(driver);
}

#[then("下一回合工具表不含 {tool:string} 且待生效标记清除")]
pub(crate) fn t_cap_next_turn_applied(capability_bdd: &CapabilityBdd, tool: String) {
    let tool = tool.trim_matches('"');
    let driver = capability_bdd.driver.borrow();
    let driver = driver.as_ref().expect("capability driver mounted");
    assert!(
        !driver.tool_names_for_test().iter().any(|name| name == tool),
        "capability: 下一回合工具表仍含 `{tool}`"
    );
    assert!(
        !driver.capability_pending(),
        "capability: 应用后待生效标记须清除"
    );
}

#[given("会话 A 禁用能力 {id:string}")]
pub(crate) fn g_cap_session_a_disabled(
    agent: &AgentState,
    capability_bdd: &CapabilityBdd,
    id: String,
) {
    let id = id.trim_matches('"');
    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = assemble_driver(runtime, store);
    driver.set_capability(id, false).expect("set_capability");
    assert!(driver.capability_pending());
    *capability_bdd.driver.borrow_mut() = Some(driver);
}

#[when("新会话 B 读取能力快照")]
pub(crate) fn w_cap_session_b_snapshot(agent: &AgentState, capability_bdd: &CapabilityBdd) {
    let (runtime, store) = make_agent_with_store(agent);
    let driver = assemble_driver(runtime, store);
    *capability_bdd.snapshot.borrow_mut() = Some(driver.capability_snapshot());
    *capability_bdd.driver.borrow_mut() = Some(driver);
}

#[then("会话 B 的 {id:string} 仍为启用且无待生效标记")]
pub(crate) fn t_cap_session_b_unaffected(capability_bdd: &CapabilityBdd, id: String) {
    let id = id.trim_matches('"');
    let snapshot = capability_bdd.snapshot.borrow();
    let snapshot = snapshot.as_ref().expect("capability snapshot");
    let entry = snapshot
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .expect("capability entry");
    assert!(entry.effective_enabled, "capability: 新会话不应继承禁用");
    assert!(!entry.session_disabled);
    assert!(!entry.pending);
    assert!(!snapshot.pending);
}
