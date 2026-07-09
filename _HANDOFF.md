# _HANDOFF — xylitol TUI（交接笔记，非规范）

> 最后更新：2026-07-09
> 分支：`feat/tui-dev`
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界以各层 `AGENTS.md` 与 skills 为准。

---

## 〇、当前阶段（读这段即可开工）

| 已完成 | 进行中 / 下一步 |
|---|---|
| `packages/xylitol-tui`：pi-tui 可移植模块初步 port + 五层测试 + `agent_demo` | 按需裁剪包；补应用层所需 API（如 `Container` / `OverlayHandle`） |
| 旧 `src/app/tui` in-tree engine/widgets **已删除**，`run()` 占位报错 | 基于 `xylitol-tui` **从零**重做产品 TUI（不继承旧 UI/UX） |
| AGENTS 收成稳定边界；how-to 进 `write-tui` / `test-tui-harness` | 开 SDD（建议 `c445` 裁剪/API → `c450` App Shell） |

**架构（已写入 AGENTS，此处不重复长文）**：引擎同步 + 应用面 host 驱动异步合流；样式 `Vec<String>`；主题闭包在包、语义 token 在应用面；流式业务缓冲在应用面。

**SSOT 指针**

- 边界：`packages/xylitol-tui/AGENTS.md`、`src/app/tui/AGENTS.md`、`src/AGENTS.md`、根 `AGENTS.md`
- How-to：`write-tui`、`test-tui-harness`、`write-surface`
- 复核草稿：`packages/xylitol-tui/REPORT.tmp.md`（工作笔记，可删，勿当规范）

**pi 源**：`../pi/packages/pi-tui`、`../kimi-code/packages/pi-tui`

---

## 一、包侧快照（易变，以 `cargo test` 为准）

- 包测试：`cargo test -p xylitol-tui`（五层 1–4 in-process）
- E2E：`just test-tui-e2e`（`#[ignore]`，PTY/tmux + `agent_demo`）
- 有意不移植：stdin-buffer（crossterm）、native-modifiers、Apple/Windows 专属输入等——见包 `AGENTS.md`

历史变更摘要（已 archive / 已提交）：阶段 0–4 port；c405–c430 harness/协议/paste-burst/autocomplete/editor；c440 `agent_demo`；旧应用面移除（占位）。

---

## 二、建议下一刀

1. **`c445`（包）**：裁剪未用图片等；补 `Container` / `OverlayHandle`（及按需 `InputListener`）。验证：`test-tui-harness` + `cargo test -p xylitol-tui`。
2. **`c450`（面）**：`write-surface`（先 `audit-dead-code`）→ 新 App Shell：`tokio` 合流 + host 驱动 `xylitol_tui`；UX 从零设计。保留 `Driver` / `dispatch` / `composition` / `XyEvent`。
3. 缺底层能力 → **先改包再接线**（见 `src/app/tui/AGENTS.md`）。

---

## 三、SDD 习惯

```
/llman-sdd-propose <id>   # c445+
# 实现…
just qa
/llman-sdd-archive <id>
git commit
```

范例：`llmanspec/changes/archive/2026-07-08-c415-port-paste-burst/`
