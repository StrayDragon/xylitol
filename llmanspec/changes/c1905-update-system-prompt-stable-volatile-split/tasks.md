# Tasks: c1905-update-system-prompt-stable-volatile-split

> Specs landing 须在 `change start` / attach 之后。本文件为 Designed 规划壳。

## 0. Review 门

- [x] 0.1 一手深挖：[`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)（日界 / skills / instructions）
- [x] 0.2 Open Questions 钉入 design（D1–D5）；proposal Out of scope `tool_search→c1960`
- [ ] 0.3 Branch binding：干净树 + 默认分支 → `llman sdd change start c1905-…`（或已有 feature 分支则 `attach`）

## 1. Specs landing（绑定分支后）

- [ ] 1.1 `agent-prompt`：片段标签语义 + date 三态（钉死 / 今日 / Omit）；稳定字节以 unit / `feature: false` 为主
- [ ] 1.2 `agent-runtime`：`ar33` date_placement 从占位升真语义；默认 `SystemPinnedAtSession`
- [ ] 1.3 **跳过** YAML / `runtime-config` 新键（code-first）
- [ ] 1.4 校验：`llman sdd validate c1905-update-system-prompt-stable-volatile-split --strict --no-interactive`（及触及 specs）

## 2. 实现（apply）

- [ ] 2.1 `DatePlacement` 扩容：`SystemPinnedAtSession` / `SystemAsToday` / `Omit`；`defaults.rs` → pin
- [ ] 2.2 session/capabilities：首次组装钉日历日；rebuild / resume 同 session 复用 pin
- [ ] 2.3 `build_system_prompt`（或等价）消费 `date_placement` + pin；文档化片段顺序；cwd 与 date 分轨
- [ ] 2.4 单测：稳定字节、跨伪日界 pin、Omit、instructions 不双写（Assembler 回归）
- [ ] 2.5 边界：不实现 StatusBar / search / observability；skills 顺序本波不强制重排

## 3. 校验

- [ ] 3.1 触及 `agent/prompt` + `context_policy` 单测
- [ ] 3.2 `just qa`（本 change 不改 live-provider 行为预期）
