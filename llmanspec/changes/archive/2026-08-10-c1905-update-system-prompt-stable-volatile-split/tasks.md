# Tasks: c1905-update-system-prompt-stable-volatile-split

> Specs landing 须在 `change start` / attach 之后。

## 0. Review 门

- [x] 0.1 一手深挖：[`research/stable-volatile-split-2026.md`](./research/stable-volatile-split-2026.md)
- [x] 0.2 Open Questions 钉入 design（含 2026-08-10 session_env 出 system 修订）
- [x] 0.3 Branch binding：`change start`

## 1. Specs landing

- [x] 1.1 `agent-prompt`：stable / session_env / Omit 默认；首轮 Env→user
- [x] 1.2 `agent-runtime`：`ar33` date_placement 默认 Omit
- [x] 1.3 跳过 YAML 新键
- [x] 1.4 全量 validate（含 BDD）与 `just qa`

## 2. 实现

- [x] 2.1 `DatePlacement` 三态；默认 **Omit**
- [x] 2.2 system 默认无 date/cwd；session_env 模块 + 首轮/resume 追加
- [x] 2.3 ReAct user 落盘前插入 session_env
- [x] 2.4 单测：Omit、append 规则、react 投影顺序
- [x] 2.5 不实现 StatusBar / search / compact-ensure（→ c1895 / c1906）

## 3. 校验

- [x] 3.1 触及 prompt / context_policy / react 单测
- [x] 3.2 `just qa`
