# 临时追踪（阶段：对齐 draft + QA + 坏味道）

> **用完即删。** 不进 AGENTS / 永久文档。落地并 commit 后删除本文件。

## 目标顺序

1. [ ] purpose-draft 提案 batch + depends_on DAG → commit
2. [ ] 文档 cleanup（过时 architecture / 空壳 TUI 表述等）→ commit
3. [ ] `llman sdd archive freeze` 旧 archive → 7z → commit
4. [ ] `llman-sdd-specs-compact`：以代码为 SSOT 重写 specs → commit
5. [ ] 坏味道分析 + 重构准备笔记（可进 draft / 本文件末节）
6. [ ] 删除本 `_TODO.md`

## Draft change 清单（purpose-draft，仅 proposal）

| ID | 主题 | depends_on |
|---|---|---|
| c1080-update-infra-mcp-client-product | MCP client 产品补齐（stdio / url·sse 等） | [] |
| c1085-update-agent-skills-runtime | Skills 加载进运行时 + 视觉表现 TBD | [] |
| c1090-add-runtime-keybindings-hot-reload | Keybindings 可热重载资源缝 | [] |
| c1095-update-runtime-theme-hot-reload | Themes 热重载 | [] |
| c1100-update-runtime-context-hot-reload | Context（AGENTS.md 等）热重载 | [] |
| c1105-add-app-tui-trust-slash | `/trust` | [] |
| c1110-add-app-tui-history-copy-last | `/history-copy-last` | [] |
| c1115-add-app-tui-theme-slash | `/theme` | [c1095] |
| c1120-add-app-tui-reload-slash | `/reload`（订阅通知、不改历史） | [c1080,c1085,c1090,c1095,c1100] |
| c1125-add-app-tui-at-path-completion | `@` 文件模糊引用 | [] |
| c1130-add-app-tui-dollar-skill | `$skill-name`（非 `/skill:`） | [c1085] |
| c1135-add-app-tui-startup-header | 启动 header：skills/mcp（不做 prompt） | [c1080,c1085] |
| c1140-add-package-tui-thinking-level-chrome | 包/demo：thinking 边框能力 | [] |
| c1145-update-runtime-model-thinking-levels | 模型配置 high/xhigh/max | [] |
| c1150-add-app-tui-thinking-level | 产品 thinking 边框/footer | [c1140,c1145] |
| c1155-add-app-tui-paste-image | 粘贴图片（含临时路径 fallback） | [] |
| c1160-update-app-tui-paste-collapse | 长粘贴 `[paste #N +lines]` 产品接线 | [] |

```text
c1080 ──┐
c1085 ──┼── c1120 /reload
c1090 ──┤
c1095 ──┤── c1115 /theme
c1100 ──┘
c1080 + c1085 ── c1135 header
c1085 ── c1130 $skill
c1140 + c1145 ── c1150 thinking app
```

## 刻意不做（写入 draft Out of scope）

- `/skill:name` slash（改 `$skill-name`）
- prompt templates 产品面 / header（用 skill 代替）
- `/settings` 运行时板（thinking 另入口）
- Extensions / Packages 市场
- OAuth `/login`

## QA / compact 备注

- validate `--all` 可能因 base ref staleness 噪；单 change / `--specs` 为准
- freeze：`--before 2026-07-14 --keep-recent 15`（dry-run 见过）
- compact：代码 SSOT；不匹配 → 报告不硬改行为

## 坏味道（待填）

- host/root ~1160 行；effects ~800；bridge ~975
- `agent/prompt/commands.rs` 双词表（pi 短名 vs `session-*`）
- docs architecture 仍写 TUI 空壳
