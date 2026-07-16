# 临时追踪（阶段：对齐 draft + QA + 坏味道）

> **用完即删。** 不进 AGENTS / 永久文档。落地并 commit 后删除本文件。

## 目标顺序

1. [x] purpose-draft 提案 batch → `fc09ec2`
2. [x] 文档 cleanup → `97e6ee3`
3. [x] archive freeze → `d71b608`
4. [x] specs-compact（本批）→ 待 commit
5. [x] 坏味道分析（见下）
6. [ ] 删除本 `_TODO.md`

## Specs compact 报告（代码 SSOT）

### 已重写
- `agent-runtime`：41→18 req；删除 adk/XyRunner/AgentEvent/OutputGuard 幽灵合约
- `agent-prompt`：去掉 Jinja2；钉 skills 注入 + 产品不以 prompt 为 header
- `cli-entry`：产品 `session-*` vs 死短名表；rpc 保持移除
- `infra-mcp`：purpose + 收窄 valid_scope；stdio/url
- `app-tui-session-tree`：去掉 stub purpose；ast2 不再写「fold 后续」
- TBD purpose 批量中文化：layer-architecture / test-* / package-tui-* / infra-provider-trace / hooks / resource-discovery
- `agent-hooks` purpose 中文化；layer 缝去掉 rpc.rs

### 仍不匹配 / 待后续（未硬改行为）
| 项 | 说明 |
|---|---|
| `agent/prompt/commands.rs` 22 短名表 | 与产品 TUI 双词表；spec 已允许 MAY 保留，代码仍 `dead_code` |
| `SlashCommandSource::extension` | enum/注释仍提 extension；产品无扩展平台 |
| `layer-architecture` / 多数 package specs | 英文 statement 仍多；本批未全量中文化 |
| `app-tui` 单体 | 仍存在；c450 退役未完 |
| BDD features | 未删（仍有绑定）；无确认死场景 |

## 坏味道 / 重构准备

| 味道 | 位置 | 建议下一刀 |
|---|---|---|
| God 文件 | `host/mod.rs` ~1160、`layout/root.rs` ~1160、`effects` ~800、`bridge` ~975 | 续拆：pending/effects 子模块；bridge handlers 已有可再切 |
| 双 slash 词表 | `agent/prompt/commands.rs` vs `app/tui/commands.rs` | 升格 draft 或 quick：收敛 GetCommands / 删死表 |
| Stub 槽冻结 | Plate/Settings/Choice | 保持冻结；勿扩 |
| 文档指针 `_HANDOFF` | 已清 live 指针 | archive 内可忽略 |
| 观测 | fastrace+log 已单栈 | 保持；禁 tracing 回潮 |

重构 draft 候选（未建）：`c1170-refactor-app-tui-host-split`、`c1175-refactor-slash-command-ssot`。


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
