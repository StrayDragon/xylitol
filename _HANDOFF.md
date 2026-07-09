# _HANDOFF — xylitol-tui：pi-tui Rust 移植与应用面接线

> 最后更新：2026-07-09
> 分支：`feat/tui-dev`
> 阶段：**port 完成 → review/裁剪 → 基于 xylitol-tui 从零重做 `src/app/tui/`**

---

## 〇、接手必读（30 秒）

**目标**：`packages/xylitol-tui` 作为通用 TUI 库已完成初步 port；下一步是裁剪/补 API，然后 **作废旧应用面实现**，基于本 package **从零设计** `src/app/tui/`（不继承旧 UI/UX）。

**怎么工作**：每个任务一个 llman SDD 变更。测试走 c405 五层（见 `packages/xylitol-tui/AGENTS.md`）。pi 源：`../pi/packages/pi-tui`、`../kimi-code/packages/pi-tui`。

**已定架构（写入 AGENTS，预想能力不写）**：

- 引擎同步；产品面 host 驱动（`dispatch_input` / `try_render` / …）。
- 异步事件合流在应用面；`TUI::start()` 仅 demo。
- 样式 `Vec<String>`；主题闭包在包、语义 token 在应用面；流式业务缓冲在应用面。
- 旧 `src/app/tui` 实现删除并占位，待重做。

详细边界 SSOT：`packages/xylitol-tui/AGENTS.md`、`src/app/tui/AGENTS.md`、`write-tui` skill。

---

## 一、当前进度

| 指标 | 数值 |
|---|---|
| 源码行数 | ~12k（`packages/xylitol-tui/src`） |
| 测试 | 255 全绿（`cargo test -p xylitol-tui`） |
| E2E | ignored PTY/tmux（`agent_demo`）；`just test-tui-e2e` |
| 应用面 | 旧实现移除，占位 `run()`；待基于 xylitol-tui 重做 |

### 已完成变更（摘要）

| 变更 | 内容 |
|---|---|
| 阶段 0–4 | 可移植模块 port + doRender 管线 |
| c405–c430 | 五层 harness / terminal 协议 / paste-burst / autocomplete / editor |
| c440 | examples → 单一 `agent_demo` |
| stdin-buffer | 不移植；crossterm 替代 |

---

## 二、跳过项与裁剪

跳过项、裁剪策略、待补 API（`Container` / `OverlayHandle` / `InputListener`）的 SSOT 在 `packages/xylitol-tui/AGENTS.md`，此处不重复。

复核细节见 `packages/xylitol-tui/REPORT.tmp.md`（工作笔记，勿当规范）。

---

## 三、llman SDD

```
/llman-sdd-propose <id>
# 实现…
just qa
/llman-sdd-archive <id>
git commit
```

- change id：`c{priority}-{verb}-{subject}`，继续 `c445+`
- 范例：`llmanspec/changes/archive/2026-07-08-c415-port-paste-burst/`

---

## 四、测试

见 `packages/xylitol-tui/AGENTS.md`（五层 + 终端矩阵）。时序禁止 `thread::sleep`。

---

## 五、下一步（应用面）

1. 裁剪 package + 补 `Container` / `OverlayHandle`（建议 `c445`）。
2. 新建 SDD 重做 `src/app/tui/`（建议 `c450`）：异步 App Shell + `xylitol_tui` 驱动面；走 `write-surface`（先 `audit-dead-code`）。
3. UI/UX **从零设计**，不以旧 `src/app/tui` 为参考。跨面 seam（`Driver` / `dispatch` / `composition` / `XyEvent`）保留。
