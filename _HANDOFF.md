# _HANDOFF — xylitol TUI（交接笔记，非规范）

> 最后更新：2026-07-09
> 分支：`feat/tui-dev`
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界以各层 `AGENTS.md` 与 skills 为准。

---

## 〇、战略方向（读这段即可对齐）

两条轨**并行**，职责分开：

| 轨 | 目标 | 真值落点 |
|---|---|---|
| **A. 包潜力 / 原型 UX** | 在 `packages/xylitol-tui` + `agent_demo` 里把**真实 `src/app/tui` 可能需要的交互原型**做全、做稳（补全、槽替换、可展开块、光标策略、host API…） | 包代码 + `agent_demo` + 五层测试；刻意差异见 `packages/xylitol-tui/PI_DELTAS.md` |
| **B. 产品面接线** | 完成 `src/app/tui` 与现有 `agent` / `infra`（经 `app/core` seam：`Driver` / `dispatch` / `composition` / `XyEvent`）的整合 | `src/app/tui/` + `write-tui` / `write-surface`；视觉 `DESIGN.md` |

**原则**

1. **先在 `agent_demo` 验证 UX/UI**，再搬进 `src/app/tui`——demo 是产品壳的交互沙盒，不是最终产品。
2. 缺底层能力 → **先改包**（通用、可扩展），再接线应用面（见 `src/app/tui/AGENTS.md`）。
3. 对照 `../pi/packages/tui` 整合时，**以 `PI_DELTAS.md` 为准**，禁止静默回退 xylitol 决议（crossterm 输入、`CompletionSource`、host 驱动等）。
4. 包 **零引用**主 crate；产品状态机 / slash 语义 / session **不进** `xylitol-tui`。

---

## 一、当前阶段

| 已完成 | 进行中 / 下一步 |
|---|---|
| 引擎 port + 五层测试 + c445（Container / OverlayHandle / Image 裁剪） | **轨 A**：继续用 `agent_demo` 挖包潜力（补全范式、键位、布局原型…） |
| `CompletionSource` 注册表：`/` slash + `@` path；Editor 无硬编码触发 | **轨 B**：`src/app/tui` 仍为占位；并行推进与 agent/infra 的 seam 整合（`c450` App Shell 仍适用） |
| `agent_demo`：单列栈、Ctrl+P/S 槽替换、thinking/tool 可展开、流式 typewriter | 展开策略（默认折叠哪些、是否记住）→ 产品接线后再锁 |
| `PI_DELTAS.md` 台账 | 每次刻意差异变更同步更新该表 |

**架构（已写入 AGENTS，此处不重复）**：引擎同步 + 应用面 host 驱动异步合流；`Vec<String>` ANSI；主题闭包在包、语义 token 在应用面；流式业务缓冲在应用面。

**SSOT 指针**

- 边界：`packages/xylitol-tui/AGENTS.md`、`src/app/tui/AGENTS.md`、`src/AGENTS.md`、根 `AGENTS.md`
- **vs pi 差异**：`packages/xylitol-tui/PI_DELTAS.md`
- How-to：`write-tui`、`test-tui-harness`、`write-surface`
- 视觉/UX：`src/app/tui/DESIGN.md`
- 分析快照（非规范）：根 `_REPORT.md`

**pi 源**：`../pi/packages/tui`（及 kimi-code 同源）

---

## 二、`agent_demo` 键位（应用级，避 Editor 冲突）

| 键 | 作用 | 备注 |
|---|---|---|
| `/` … | Slash CommandPopup（`SlashCommandSource`） | 编辑流内嵌 SelectList |
| `@` … | 路径补全（`AtPathSource`） | 同上 |
| `Ctrl+P` / `Ctrl+S` | palette / settings **替换 editor 槽** | 非 overlay |
| `Ctrl+T` | 展开/折叠 thinking | |
| **`Alt+E`** | 展开/折叠 tools | **不用 Ctrl+E**（= `tui.editor.cursorLineEnd`） |
| **`Alt+G`** | 切换 glyph 档 | **不用 Ctrl+G**（预留外部 editor） |
| `Ctrl+O` | 步进脚本 | |
| `Esc` | 关 palette/settings；或关补全 popup | |
| `Ctrl+C` | 退出 | |

---

## 三、包侧快照（易变，以 `cargo test` 为准）

- 包测试：`cargo test -p xylitol-tui`（五层 1–4）
- E2E：`just test-tui-e2e`（`#[ignore]`，PTY/tmux + `agent_demo`）
- 有意不移植：见 `AGENTS.md` + `PI_DELTAS.md`（stdin-buffer、native-modifiers、完整 Image encode…）

---

## 四、建议下一刀

### 轨 A（包 / demo）— 优先挖潜力

1. 在 `agent_demo` 继续补**产品面将需要的原型**：更多 `CompletionSource` 示范（文档级 `$`/`^`）、确认 overlay、长 transcript + 槽替换回归、复制友好规则等。
2. 缺通用能力 → 进包（trait / 组件），**不要**在 demo 里复制准通用实现。
3. 对照 pi 时更新 `PI_DELTAS.md`，勿无 pi 产品壳覆盖 xylitol 库边界。

### 轨 B（产品面）— 并行整合

1. **`c450`（面）**：`write-surface`（先 `audit-dead-code`）→ App Shell：`tokio` 合流 + host 驱动；`Container` 组栈；`OverlayHandle`；UX 对齐 `DESIGN.md`；交互模式从 `agent_demo` **迁移**而非重发明。
2. Agent/infra 只经 `app/core` seam；扩行为先扩 `runtime_protocol` / `agent`。
3. （可选）Image encode feature-gate，非默认路径。

---

## 五、SDD 习惯

```
/llman-sdd-propose <id>   # c450+ 或 package-tui-*
# 实现…
just qa
/llman-sdd-archive <id>
git commit
```

范例：`llmanspec/changes/archive/2026-07-09-c445-add-package-tui-container-overlay/`
