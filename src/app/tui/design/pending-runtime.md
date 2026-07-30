---
version: "alpha"
name: "pending-runtime"
description: "NextTurn pending chrome — active in footer; next-turn cue on agent-busy row; revert/idle converge."
tokens_from: "../DESIGN.md"
components:
  pending-cue:
    textColor: "{colors.muted}"
---

# Pending runtime（NextTurn · 下轮预告）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 产品意图：[`docs/architecture/运行时即时设置.md`](../../../../docs/architecture/运行时即时设置.md)。
> 固定词：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。
> 下轮预告槽：[`status.md`](./status.md)。Footer：[`footer.md`](./footer.md)。
> 静图：[`playground/`](./playground/)「Pending / NextTurn」槽 + Full shell 芯片。
> 贴输入 MCP（**已落地 c1210**）：主面 `/mcp`；idle 时若 sticky 头卡已有 `mcp:` 行则 **不**画短 cue；busy 可用右对齐 `mcp pending (see /mcp)`（[`mcp-input-cue.md`](./mcp-input-cue.md)）。busy 且已有 `Next turn…` 时 MCP cue **不**覆盖。

## 产品意图

agent **还在跑**（含 in-flight 模型流）时用户换了模型 / thinking：

- **输入框下方 footer** 仍显示 **生效中**（当前 turn / in-flight 实际在用的值）；
- **agent-busy status 行右侧**（**下轮预告** / next-turn cue）dim 显示待生效接替；
- 再选回生效中 → 下轮预告 **消失**，恢复如初；
- **不**靠 scrollback **滚动提示**解释「何时生效」。

idle / 无 in-flight：切换直接写入生效中（无下轮预告）；footer 更新；仍无成功滚动提示墙。

## 词汇

| 词 | 含义 |
|---|---|
| **选中（selected）** | 会话当前选定的模型 / thinking（`select_model` / cycle 后） |
| **生效中（active）** | 本 agent run **实际握住**的 in-flight / 本 turn 请求所用值（≠ 仅读 ModelManager） |
| **待生效（pending）** | `selected ≠ active`；仅此时画下轮预告 |
| **下轮预告（next-turn cue）** | status 行右侧 dim 文案（常为 `Next turn: …`） |

## MUST

1. **双态**：`pending` 时 MUST 同时可见 active（footer）与下轮预告；MUST NOT 只把 footer 改成新模型而暗示当前流已换。
2. **Footer = active**：有 pending 时 footer 的 model / thinking MUST 反映 **active**；MUST NOT 用 selected 覆盖 footer 主字段。
3. **下轮预告**：仅 **agent run busy** 时，与 status **同一行**右侧（`.status-next-turn-cue`）；左侧 **`.status-lead`** = `spinner + 短词` 一体、**MUST NOT** 把短词右对齐到行尾。见 [`status.md`](./status.md)。
4. **仅 bang busy**：MUST NOT 画 `Next turn:` 下轮预告；换模 / thinking MUST 按无 in-flight——footer 跟选中，无 pending 态。
5. **下轮预告文案（M0 · 互斥优先级，非拼接）**：模型与 thinking **均只经 `/model` picker** 提交（无全局 Shift+Tab）。busy 内先后提交两次（或一次提交两者但仅模型变化 / 仅 level 变化）时，下轮预告 **只显示一项**（优先级：模型 > thinking）：

   | 情况 | 下轮预告 |
   |---|---|
   | 模型 pending（无论 thinking 是否也 pending） | `Next turn: {model}` |
   | 仅 thinking pending（同模型改 level） | `Next turn thinking: {level}` |
   | 皆无 | （无下轮预告） |

   **MUST NOT** 使用 `Next turn: {model} · {level}` 拼接。等级文案仅为 **xylitol 档名**。窄宽优先截断下轮预告。
6. **清除 / 收敛**：
   - `selected == active`（含用户切回）→ 清下轮预告；
   - **同 run 下一 turn** 消费 → active←selected，清下轮预告；
   - **AgentEnd / abort 落定 / 进入 idle** 且无 in-flight → active 与 selected **收敛**，清下轮预告（补「末 turn 换模后无下一 turn」洞）。
7. **成功路径**：chrome 已表达时 MUST NOT 追加 muted 滚动提示确认行。
8. **与队列共存**：steer/follow-up 在 queue 带；下轮预告在 status 右；互不合并。
9. **静图 / fixture**：playground 三态——仅模型 / 仅 thinking / cleared；fixture `pending-model|thinking.next-turn`；`check_tui_design_playground` 绿。

## 形状（固定 · 下轮预告）

**恒定示例 active**：footer = `ornith · thinking off`。

### 仅模型 pending（`/model` 之后）

```
⠋ Working                    Next turn: deepseek-v4-flash
~/xylitol · ornith · thinking off
```

### 仅 thinking pending（`/model` 同模型改 level）

```
⠋ Working                    Next turn thinking: high
~/xylitol · ornith · thinking off
```

### 一次 Enter 提交 model+level（模型优先）

若模型与 level 相对 active 都变了 → 下轮预告 **只** `Next turn: {model}`（level 一并在 NextTurn 消费，不拼接）。

### cleared / 切回 / idle 收敛

无下轮预告。

静图芯片：playground「Pending / NextTurn」→ **仅模型** / **仅 thinking** / **cleared**；Models 槽见 level 列。

## 防回归 / 反例（实现必测意识）

| 风险 | 要求 |
|---|---|
| active 误绑 ModelManager | footer/下轮预告以 **in-flight / run 握住的值** 为 active；选中单独存 |
| 末 turn 换模后无下一 turn | idle/abort **必须收敛**，禁止幽灵 pending |
| status 短词轮换丢下轮预告 | 下轮预告与 lead 短词解耦；compact/retry/tool 后仍在 |
| 窄宽涨行 | 截断下轮预告；status 内容仍一行 |
| 仅 bang 画 Next turn | 禁止；无 LLM 边界语义 |
| 多类 pending 炸槽 | 模型优先于 thinking；**禁止** `model · level` 拼接；能力覆盖不进 M0 |
| 差分闪烁 | 下轮预告变更勿重置 spinner 相位；勿整壳无故重排 |
| 滚动提示墙回流 | 成功换模禁止 `model →` scrollback |

## 不做

- footer `a → b` 作为唯一待生效通道。
- queue 带再占一行 `Next turn:`。
- Settings / Plate；Scope `all|scoped`；能力覆盖进 M0 下轮预告。
- 把 pending 写成 scrollback 滚动提示墙。

## 验收指针

- playground：三芯片；legend 标明 selected vs active；无拼接态。
- Full shell：换模下轮预告（模型态）+ agent busy；可与队列同开。
- 落地 harness：footer=active；agent busy 含下轮预告；切回 / idle 清；status 轮换保下轮预告；bang-only 无下轮预告。
