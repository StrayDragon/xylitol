---
depends_on: []
branch: sdd/c2780-add-command-execution-class
base_sha: 1286d3c5df5b744fb05ad8e97c5a4ace59d78b1c
checkpointed: false
---

# 命令执行类声明：阻塞语义进 Command SSOT，泵循环泛型消费

## Why

qa-e2e 修复过程（commit `11c70806` 前的诊断）实测暴露：交互 bang 期间 `/model` 等 slash 的 effect
不生效——`run_interactive_bang` 接管会话循环后只泵 input/tick/chunk，不调 `drain_pending`，
`/model` 的 OpenModels arm 一直排队到 bang 结束（`!sleep 30` 实测挂载延迟可达 30s 超时）。
c2740 Further Notes 已把「Host 泵下沉」记为 follow-up；本 change 把它从"补丁"升格为
**机制**：让每条命令的执行语义（是否独占循环、是否可随时生效）成为 Command 机制的一等声明，
而不是散落在循环硬编码与逐命令特判里。

今天的语义分裂在三处、且互不连通：

1. **Host 侧**：`protocol/wire/registry.rs` 每方法一行 `Auth::Writer|Readonly`（租约准入）——
   只回答"能否与写者并发"，不回答"客户端何时执行它"。
2. **Client 侧**：`BusySlashPolicy`（Allow/Reject）——只回答"busy 时允许提交吗"，
   Allow 的命令在 bang 期间照样排队。
3. **循环结构**：主循环每轮泵 `drain_pending`（agent-busy 下 effects 正常）；bang/reload
   交互循环接管后不泵——哪些命令被阻塞由"哪个循环在跑"决定，命令本身无法声明。

结果即用户三问的现状答案：表达不了（语义散落）、扩展不简单（新命令要摸清循环结构）、
"阻塞哪些命令"没有单一事实源。

## What Changes

- **执行类（execution class）进 Command SSOT**：在 wire 注册表每方法属性行上新增
  `Exec` 维度（与既有 `Auth/Idem/Resp/command_backed` 并列），客户端 `PendingSlash`/
  effects 侧同源消费。初版分类：
  - `Exclusive`——独占会话循环直至完成（`bash`、`prompt`、`reload`）；
  - `Inline`——任何循环（含 bang/reload select）内可立即执行生效
    （`get_state`、`get_available_models`、`set_model`、`set_thinking_level`、
    `queue_stats`、`get_commands`、`loaded_resources` 等读/轻写命令）；
  - `Queued`——保持现状排队语义（其余会话操作类命令，行为不变）。
- **泵循环泛型消费**：`run_interactive_bang` / `run_interactive_reload` 的 select 增加
  effects 泵臂，仅执行声明为 `Inline` 的 pending 命令；`Exclusive`/`Queued` 仍等循环归还。
  主循环行为不变。
- **新命令扩展路径**：新增一条命令 = `Command` 变体 + REGISTRY 行（含 Exec 标注）+
  host 派发臂 + effects arm；执行语义随行声明，无需改动任何循环结构。
  （本 change 同时以 `get_available_models` 为样板验证该路径。）

## 非目标

- 不改 Host 侧租约/锁粒度（`Auth::Readonly` 在写者期间的 `slot.driver` 争用是独立问题，
  另行立项）。
- 不改 agent-busy 主循环的泵行为（已经正确）。
- 不改 c2770 的 interrupted 提示投影。
- 不做 slash 完成弹窗 / UI 词汇调整。

## Capabilities

- `app-tui-commands`：命令执行类声明与消费规则（新 Rule + 可执行场景）。
- `app-tui-host`：交互循环（bang/reload）泵 Inline 命令的循环契约（新 Rule + 场景）。

## Impact

- 用户可感知：bang 运行期间 `/model`、`/thinking` 等即开即用、即关即走；不再出现
  "浮层迟到挂载无视 Esc"一类的竞态土壤。
- 风险：bang select 增加 effects 臂后，Inline effect 若阻塞会拖慢 bang 输出泵——
  design 约束 Inline effect MUST 非阻塞（缓存/本地直出），注册表声明即承诺。
- 兼容：`Queued` 类保持现状，既有场景零行为变化；守卫测试锁定 REGISTRY 行数与
  分类完备性。
