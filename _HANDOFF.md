# _HANDOFF — xylitol TUI（交接笔记，非规范）

> 最后更新：2026-07-09
> 分支：`feat/tui-dev`
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界以各层 `AGENTS.md` 与 skills 为准。

---

## 〇、当前阶段（读这段即可开工）

| 已完成 | 进行中 / 下一步 |
|---|---|
| `packages/xylitol-tui`：pi-tui 引擎参考 port + 五层测试 + `agent_demo` | **`c445` 已 archive**（Container / OverlayHandle / Image 裁剪） |
| `agent_demo`：极简单列；Ctrl+P/S 替换 editor 槽 | **`c450`**：App Shell；UX 见 `src/app/tui/DESIGN.md` |
| 输入硬切：`InputEvent::{Key,Paste}` | propose `c450`（depends on c445） |
| 包 API：`Container`、`OverlayHandle`；`is_image_line` 保留 | — |

**架构（已写入 AGENTS，此处不重复长文）**：引擎同步 + 应用面 host 驱动异步合流；样式 `Vec<String>`；主题闭包在包、语义 token 在应用面；流式业务缓冲在应用面。

**SSOT 指针**

- 边界：`packages/xylitol-tui/AGENTS.md`、`src/app/tui/AGENTS.md`、`src/AGENTS.md`、根 `AGENTS.md`
- How-to：`write-tui`、`test-tui-harness`、`write-surface`
- 视觉/UX：`src/app/tui/DESIGN.md`

**pi 源**：`../pi/packages/tui`（及 kimi-code 同源）

---

## 一、包侧快照（易变，以 `cargo test` 为准）

- 包测试：`cargo test -p xylitol-tui`（五层 1–4 in-process）— c445 后绿
- E2E：`just test-tui-e2e`（`#[ignore]`，PTY/tmux + `agent_demo`）
- 有意不移植：stdin-buffer（crossterm）、native-modifiers、Apple/Windows 专属输入等——见包 `AGENTS.md`
- 图片：已删 `Image` 组件与 Kitty/iTerm encode；保留 `is_image_line` + `hyperlink`

---

## 二、建议下一刀

1. **`c450`（面）**：`write-surface`（先 `audit-dead-code`）→ App Shell：`tokio` 合流 + host 驱动；用 `Container` 组 transcript/status/editor；overlay 用 `OverlayHandle`；UX 对齐 `src/app/tui/DESIGN.md`
2. 缺底层能力 → **先改包再接线**（见 `src/app/tui/AGENTS.md`）
3. （可选）按需恢复 Image encode 为 feature-gate，非默认路径

---

## 三、SDD 习惯

```
/llman-sdd-propose <id>   # c450+
# 实现…
just qa
/llman-sdd-archive <id>
git commit
```

范例：`llmanspec/changes/archive/2026-07-08-c415-port-paste-burst/`
