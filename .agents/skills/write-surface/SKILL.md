---
name: "write-surface"
description: "新增或改造应用面（app 层：interactive REPL、TUI、server 客户端、未来 GUI）时使用的方法论。先做死代码分诊、再按复用契约划分职责、最后经 mode 分发接线。"
---

# 新增应用面方法论（Write Surface）

当你要新增或改造 `app/` 下的一个应用面（交互式 REPL、TUI、server 客户端、未来 GUI）时，按本 skill 的流程走。它的存在理由：本项目历史上积累了大量 dead code，根因是「先搭骨架留待将来」而复用契约从未被该面真正驱动。本流程把「先分诊死代码 → 只在契约内接线 → 每个面必须可端到端跑通」变成硬步骤，从源头阻止新一轮骨架腐烂。

**先读**：根 `AGENTS.md`「项目地图」→ `src/AGENTS.md`「分层不变量」→ 目标面的 `AGENTS.md`（如 `src/app/tui/AGENTS.md`）。方法论总纲见本 skill。

## 1. 复用契约（不可改写的分工）

应用面不是从头造轮子。print 模式已经证明这条路通：

```
composition::build_agent  →  InProcessDriver  →  Driver::run(prompt)
   →  XyEvent 流  →  该面自己的 renderer
```

| 层 | 处置 | 具体对象 |
|---|---|---|
| **复用，绝不重写** | 共享，所有面走同一条 | `app/core/composition.rs::build_agent`（组合根，唯一允许同时 import agent+infra，见 `src/AGENTS.md`）；`Driver` trait + `InProcessDriver`；`ReActAgent::run/run_with_id → XyEventStream`；`protocol::Command` 的语义；`domain::lifecycle::XyEvent` 变体集 |
| **每面重写，绝不共享** | 面内独占 | 输入采集（REPL 循环 / HTTP handler / 行编辑器）；渲染（stdout / TUI widget / HTTP JSON）；slash 命令 → driver 调用的本地分派 |
| **新 agent 能力** | 进 `agent/`，不进面 | 若新面需要 agent 还没有的行为，那是 agent 层的 port 扩容（先在 `runtime_protocol/` 加 trait，再在 `infra/` 实现），不是在面里 reach into `agent::session` |

判定原则：一段逻辑「任何面都需要」→ 复用侧；「只有这个面才需要」→ 重写侧；「需要 agent/infra 内部」→ 不属于面，上提到 agent/infra。

## 2. 四步流程

### 步骤 1 — 分诊死代码（先于一切动手）

在你要动的那块区域，先跑 `audit-dead-code` skill（见 `.agents/skills/audit-dead-code/SKILL.md`）。**例外**：纯合约/文档变更（仅改 `llmanspec/`、AGENTS、skills、`_tmp_prompts/`）可跳过；第一个写 `src/app/tui` 产品代码的 change（c460 起）必须跑。产出三类清单：

- **真死**（零引用）：删。
- **逻辑死**（有引用但无真实用户路径触达）：决定「激活」还是「删」。绝不在其上叠加新代码。
- **预留**（架构意图明确）：仅当本次新面会在本变更内驱动它，才保留；否则删，等真需要时再加（骨架留着就会变成下一轮逻辑死）。

铁律：**不要在未分诊的骨架上写新功能。** 先 `rg '#\[allow\(dead_code\)\]' src --type rust` 拿到实时清单再分诊。

### 步骤 2 — 确认复用边界，只改重写侧

- 面的新代码只允许 import：`crate::app::core::composition`、`crate::app::core::driver`（`Driver`/`InProcessDriver`/`RemoteDriver`）、`crate::agent`（mod 级：`ReActAgent`/`Agent`/`XyEvent`/`XyEventStream`/`AgentBuilder`）、`crate::protocol`、`crate::domain`。
- 面**禁止** import：`crate::agent::session::*`、`crate::agent::runtime::*`、`crate::infra::*` 的任何子模块。唯一例外是组合根（`cli/mod.rs`、`server/subcommand.rs`、`rpc.rs`、`core/composition.rs`），它们在构造期注入具体 adapter。
- `src/tests.rs::arch_guard` 会拦 `agent ↔ infra` 互引；但它拦不住「面 reach into agent 内部」，那是本 skill 的社会性规则，靠 review 把关。

### 步骤 3 — 经 mode 分发接线，绝不绕过 seam

新面通过 `app/cli/mod.rs` 的 mode 分发进入（对标 pi 的 `modes/index.ts`：`InteractiveMode` / `runPrintMode` / `runRpcMode`）：

```rust
match app_mode {
    Mode::Print       => print::run_print(&mut driver, &prompt, &sid).await,
    Mode::Interactive => interactive::run(&mut driver).await,   // 新面
    Mode::Rpc         => rpc::run(/* … */).await,
}
```

如果 seam（`Driver` trait 或 `XyEvent` 枚举）不足以支撑新面，**扩 seam**（给 `Driver` 加方法 / 给 `XyEvent` 加变体），而不是让面绕过 `Driver` 直接持有 `ReActAgent` 内部。扩 seam 时同步更新所有既有面。

### 步骤 4 — 每个面必须可端到端跑通，才认它「存在」

- print ✅（已通）
- interactive（REPL）：未建。建成标准 = `cargo run --` 进入 REPL，多轮对话，`XyEvent` 流式渲染，至少 `/exit` `/model` 两条 slash 命令。
- server（REST/WS）：服务端 ✅、客户端（`RemoteDriver`）🟡 预留。建成标准 = 用 `RemoteDriver` 连上 server，走完一个 prompt 的完整事件流。
- TUI 🟡（占位，基于 `xylitol-tui` 重做，见 `src/app/tui/AGENTS.md`）。细则与重做进度见 `write-tui` skill。
- GUI：空占位。

铁律：**一个面没有端到端可跑通的入口，就不算存在** —— 它会立刻开始腐烂。

## 3. 何时该「重写」而非「复用」

只在满足以下任一条件时才重写既有代码：

- 该代码跨越了上表「复用侧 / 重写侧」的边界（例如 print 的渲染逻辑被多个面需要 → 上提为 `app/render/` 共享件，而非每面复制）。
- `audit-dead-code` 判定它「逻辑死」，且复用成本 > 重写成本（罕见；多数情况应先尝试激活）。
- 它违反分层不变量（见 `src/AGENTS.md`）。

其余情况一律复用。**重写不是默认选项**：本项目已从「自研 → 模仿 pi + 参考 kimi-code → 重构」走过三轮，每轮重写都在制造新的逻辑死码。

## 4. 红线

- 在未跑 `audit-dead-code` 的区域直接新增功能。
- 新面 reach into `agent::session` / `agent::runtime` / `infra::*`（绕过 `Driver`）。
- 新面绕过 `composition::build_agent`，自己拼装 `AgentBuilder`（组合根职责重复 = 下一轮死码温床）。
- 同时铺多个面的骨架（必然产生未被驱动的逻辑死码）。
- 用 `#[allow(dead_code)]` 让新骨架「先过编译」。新面写出来必须当变更内就被真实入口驱动。

## 5. 产出与下一步

完成一个新面后：

1. 跑 `just qa`，确认 `arch_guard` 与全测试通过。
2. 在本面目录补/更新 `AGENTS.md`（参照 `src/app/tui/AGENTS.md` 的结构）。
3. 若该面有复杂的交互组件（dialog/selector/输入框），补一个该面的 `DESIGN.md`（对标 kimi-code `write-tui/DESIGN.md`）作为单一真值源。
4. 用 `/llman-sdd-propose` 把这次新增登记为一个变更，保持 specs 同步。
