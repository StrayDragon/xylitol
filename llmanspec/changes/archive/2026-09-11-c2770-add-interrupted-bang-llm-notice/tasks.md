# Tasks

> Seam（与既有 harness 对齐，不发明新缝）：
> - 单测缝 = `agent::llm_project` 纯折叠函数（`Vec<AgentMessage>` + done 集，无 IO）。
> - BDD 缝 = `tests/bdd/bindings_agent_session.rs` 的 `#[scenario(path =
>   "llmanspec/specs/agent-session/agent-session.feature")]` + 既有 fixtures
>   （`AgentState` / `Workspace` / `XySessionStore`），场景措辞对齐 as45 既有
>   「run_with_id 播种 history」步骤族。

- [x] t1: interrupted 折叠 SSOT——`agent::llm_project` 纯函数（会话级 done 集配对 +
  `[interrupted] $ <command>` 模板 + exclude 尊重 + 裁切窗口内投影）+
  接入 `load_conversation_history`（as45 播种路径）。验证：`cargo test --lib
  --all-features agent::llm_project`（orphan 投影 / done 配对不投影 / `!!` 不投影 /
  重复构建字节稳定 四组单测）。
  apply 修订：infra `build_session_context` 不可依赖 agent 层（`infra ↛ agent`），
  折叠纯函数与模板构造器落 `protocol::session::entries` + `protocol::message`
  （`EnvMessage::interrupted_bash`，双装配点同一来源，as48 单路径保持）。
  测试 5 例在 `protocol::session::entries::interrupted_bash_tests`。
- [x] t2: `build_session_context`（SessionManager）同折叠接入（as48 单路径一致性）+
  fork/travel 路径外 done 防误报单测。验证：`cargo test --lib --all-features
  infra::session`。`[blocked-by: t1]`
  测试 3 例在 `infra::session::manager::tests::interrupted_bash`（orphan 投影 /
  done 任意位置抑制 / `!!` 不投影；off-path 抑制在 t1 纯函数级覆盖）。
- [x] t3: BDD 绑定 + steps（as-bang1 两个 `@executable` 场景：孤儿 running 投影 /
  排除与 done 配对）+ 全量门禁。验证：`cargo test --lib --all-features
  tests::bdd::` + `just qa`。`[blocked-by: t1]`
  场景措辞修订：「当 经播种路径构建 LLM history」直击真实缝
  `load_conversation_history`（原「run_with_id 播种 history」与 as45 既有绑定
  撞名且只 load 原始 entries）；双构建存 `sess_interrupted::BUILDS` 供字节一致
  断言。BDD 444 全绿。
