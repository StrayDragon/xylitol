---
depends_on: []
branch: sdd/c2780-add-command-execution-class
base_sha: 1286d3c5df5b744fb05ad8e97c5a4ace59d78b1c
checkpointed: true
checkpoint_sha: 889c1c3e8f6d2fbd2c13adf2271a1c1a1fe3e828
---

# 命令执行类声明：阻塞语义进 Command SSOT（交互循环 drain 接入移交 c2790）

## Why

qa-e2e 修复过程（commit `11c70806` 前的诊断）实测暴露：交互 bang 期间 `/model` 等 slash 的 effect
不生效——`run_interactive_bang` 接管会话循环后只处理 input/tick/chunk，不调 `drain_pending`，
`/model` 的 OpenModels arm 一直排队到 bang 结束（`!sleep 30` 实测挂载延迟可达 30s 超时）。
c2740 Further Notes 已把「Host 泵下沉」（Host 泵 = 会话主循环的 drain 调度，见术语表）记为 follow-up；本 change 把它从"补丁"升格为
**机制**：让每条命令的执行语义（是否独占循环、是否可随时生效）成为 Command 机制的一等声明，
而不是散落在循环硬编码与逐命令特判里。

今天的语义分裂在三处、且互不连通：

1. **Host 侧**：`protocol/wire/registry.rs` 每方法一行 `Auth::Writer|Readonly`（租约准入）——
   只回答"能否与写者并发"，不回答"客户端何时执行它"。
2. **Client 侧**：`BusySlashPolicy`（Allow/Reject）——只回答"busy 时允许提交吗"，
   Allow 的命令在 bang 期间照样排队。
3. **循环结构**：主循环每轮调度 `drain_pending` 清算 pending（agent-busy 下 effects 正常）；bang/reload
   交互循环接管后不调度 drain_pending——哪些命令被阻塞由"哪个循环在跑"决定，命令本身无法声明。

结果即用户三问的现状答案：表达不了（语义散落）、扩展不简单（新命令要摸清循环结构）、
"阻塞哪些命令"没有单一事实源。

## What Changes

- **执行类（execution class）进 Command SSOT**：在 wire 注册表每方法属性行上新增
  `Exec` 维度（与既有 `Auth/Idem/Resp/command_backed` 并列），客户端经
  `exec_class(&Command) -> Exec`（无通配穷举 match——新命令不分类即编译失败）消费。
  初版分类：
  - `Exclusive`——独占会话循环直至完成（`bash`、`prompt`、`reload`）；
  - `Inline`——任何循环内可立即执行生效（`get_state`、`get_available_models`、
    `set_model`、`cycle_model`、`set_thinking_level`、`queue_stats`、`get_commands`、
    `loaded_resources`、`append_entry_label`、`set_session_name(_for)` 等读/轻写命令）；
  - `Queued`——保持现状排队语义（其余会话操作类命令，行为不变）。
- **新命令扩展路径**：新增一条命令 = `Command` 变体 + REGISTRY 行（含 Exec 标注）+
  host 派发臂 + effects arm；执行语义随行声明，无需改动任何循环结构。
- **交互循环 drain 接入（移交 c2790）**：apply 实测 `run_interactive_bang` /
  `run_interactive_reload` 的长时 dispatch future 横持 `&mut dyn XyDriver`
  （bang.rs:49 / reload.rs:62），tick 臂无法在不破坏 bash 语义的前提下调用任何
  需要 driver 的 effects——drain 接入需先做 bash 派发所有权倒置
  （拥有型完成接收端，镜像 agent `run() → EventStream`），已 draft 为
  `c2790-invert-bash-dispatch-ownership`（depends_on 本 change）。

## 非目标

- 不改交互循环的 drain 调度结构（见上，c2790 承接；`11c70806` 已压住用户可见症状：
  Inline 命令在 abort/结束后即时生效，浮层不再迟到挂载无视 Esc）。
- 不改 Host 侧租约/锁粒度（`Auth::Readonly` 在写者期间的 `slot.driver` 争用是独立问题，
  另行立项）。
- 不改 agent-busy 主循环的 drain 调度（已经正确）。
- 不改 c2770 的 interrupted 提示投影。
- 不做 slash 完成弹窗 / UI 词汇调整。

## Capabilities

- `app-tui-commands`：命令执行类声明与消费规则（新 Rule atm18；
  可执行场景随 drain 接入移交 c2790）。

## Impact

- 表达层：新增/改类命令的执行语义在 REGISTRY 行一行声明 + 穷举 match 一臂，
  编译器强制分类；守卫测试锁定 REGISTRY 列与推导一致。
- 兼容：`Queued` 默认类保持现状，既有场景零行为变化；本 change 不改变任何
  运行时 drain 调度行为（`11c70806` 除外，已独立合入）。

## Further Notes

- apply 阻断记录：design §3 原方案（交互循环 select 的 tick 臂直调
  `drain_inline_pending`）与借用现实矛盾（`Pin<Box<dyn Future>>` 横持
  `&mut dyn XyDriver`），详见 design §3 修订与 c2790 draft。
- c2790 draft 随本 change 分支携带（`llmanspec/changes/c2790-invert-bash-dispatch-ownership/`，
  depends_on 本 change）。
