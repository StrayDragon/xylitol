# _HANDOFF（临时交接 · 勿当长期规范）

> 生成：2026-07-14 夜。给后续开发者 / agent。稳定规则仍以根/`src` `AGENTS.md` 与 `llmanspec/` 为准；本文件可删可改。

## 一句话

本轮主线已归档 **TUI QA（c715–c730）实现 + c735 hook 对齐 pi**；抓包与 hook 后置拆成 **c995–c999 purpose-draft**。工作区改动应已 commit；未跑满 `just qa` 全绿保证。

## 已归档（今日相关）

| Change | 要点 |
|--------|------|
| `2026-07-14-c715` … `c730` | TUI QA：rstest-bdd、abort suppress、shared bang、host 拆分、busy-bang 硬拒等 |
| `2026-07-14-c735-update-agent-hooks-pi-parity` | `XyHookBus`；Responses/Anthropic HTTP 三缝；ReAct 脚本桥；session start/compact/before_fork；`agent_settled` |

主 specs 已合并：`llmanspec/specs/agent-hooks/spec.toon` 等。

## 活跃 purpose-draft（勿 apply 到未升格）

| ID | 用途 | 依赖 |
|----|------|------|
| `c995-update-agent-hooks-driver-session` | Driver：tree / switch / shutdown hooks | c735 已归档 ✓ |
| `c996-update-agent-hooks-model-select` | `model_select` / `thinking_level_select` | c735 |
| `c997-update-agent-hooks-bang-input` | TUI `user_bash` / `input` | c735 |
| `c998-update-infra-completions-provider-hooks` | Completions 完整 HTTP 三缝 | c735 |
| `c999-add-infra-provider-traffic-capture` | Track B：外挂 mitmproxy / claude-tap（**不**嵌 mitmproxy_rs） | 无硬依赖 |

细则见各 `proposal.md`；c735 后置索引：`llmanspec/changes/archive/2026-07-14-c735-*/future.md`（若 archive 内仍保留）。

## 代码真值（hook）

- 端口：`src/runtime_protocol/hook.rs` → `XyHookBus`（**agent 禁止 import infra**）
- 脚本：`src/infra/hooks/`（`HookEvent`、`dispatcher`、`http.rs` 三缝辅助）
- 装配：`composition` / `bootstrap` 注入 `Arc<HookDispatcher>` → model factory + agent `hook_bus`
- 热路径：`react.rs`（tool/context/lifecycle）；`session/mod.rs`（start/compact/before_fork）
- Completions：**仅存 hooks 句柄**，HTTP 三缝未做 → `c998`

## TUI / 手测备注

- Abort：Esc 同步臂 `suppress_xy` + 清 streaming；drain 仍 `Driver::abort` + Aborted note
- Busy + `!cmd` Enter：硬拒（不当 steer）
- 已知产品现象：部分模型轮次 **session JSONL 只有 thinking、无 text**（正文在 thinking 里）——**非 TUI 错画**；真线协议用 `c999` PoC
- `just qa` 曾见：session fork 并行 flake；`agent_demo` PTY `:palette`/`:settings` 失败（与本次 app-tui 改动无关嫌疑）
- 产品 Fake PTY bang 路径曾绿

## 建议下一步（任选）

1. 升格并 apply **c995–c998** 中一条（先 Driver session 或 Completions）
2. **c999** PoC：mitmproxy 外挂 / claude-tap 打 `tufa`，写 design 笔记再升格
3. 本地 `just qa` + 手测 TUI abort/bang
4. 查 ornith/Responses「仅 thinking」是网关还是映射漏 `output_text`

## 命令速查

```bash
llman sdd list
llman sdd show c995-update-agent-hooks-driver-session
cargo test --lib hooks
cargo test --test bdd test_hook -- --test-threads=1
just qa          # 满闸，下班前未强制全绿
```

## 勿做

- 把成功 SSE body 默认塞进 hook（对齐 pi；抓包装 c999）
- 未升格就 apply purpose-draft
- 把 `_HANDOFF.md` 内容抄进长期 `AGENTS.md`
