# Tasks

## 1. 配置字段

- [x] 1.1 `AgentProfile` 增加 `persona` / `tools`。
- [x] 1.2 `resolve_profile` 拼接 persona → 空行 → system_prompt，并返回 tools。
- [x] 1.3 `ResolvedProfile`（SDK）增加 `tools`。
- [x] 1.4 配置 crate 单测：字段反序列化、拼接顺序、tools 携带。

## 2. 选择与装配

- [x] 2.1 `BootstrapInput` 增加 `agent_profile`；显式预设缺失返回 `AgentProfileNotFound`。
- [x] 2.2 `ResolvedAssembly.tool_features.tool_selection` 采用 profile tools（非空时）。
- [x] 2.3 CLI `--agent-profile` 接入 TUI / Print surface。
- [x] 2.4 更新所有 `BootstrapInput` 字面量。

## 3. 规格与 BDD

- [x] 3.1 `runtime-config` 新增 `agent-profile-preset-fields` 规则与两个场景。
- [x] 3.2 新增 `AgentProfileBdd` step 与 binding，`cargo test --all-features --test bdd` 全绿。

## 4. 配置产物与文档

- [x] 4.1 `just gen-config-schema` / `just gen-config-example` 更新产物。
- [x] 4.2 README 增加 `--agent-profile` 与 persona/tools 说明。

## 5. 门禁与收尾

- [x] 5.1 `just qa` 全绿。
- [x] 5.2 `just qa-e2e` 全绿。
- [x] 5.3 `llman-sdd validate c3002-update-agent-presets --check --strict` 通过。
- [x] 5.4 verify 无 CRITICAL，finalize 归档；PR 用 rebase merge。
