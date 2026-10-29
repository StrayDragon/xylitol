# Tasks

## 1. LSP 插件

- [x] 1.1 新 crate `xylitol-official-plugin-lsp`：`LspPluginConfig` + `LspPlugin`。
- [x] 1.2 工具 `lsp_diagnostics` / `lsp_symbols` / `lsp_hover`。
- [x] 1.3 懒启动 lspz `AgentHandle`；卸载 shutdown。
- [x] 1.4 URI 解析与参数校验。
- [x] 1.5 单测 + basedpyright ignored roundtrip。

## 2. DAP 插件

- [x] 2.1 新 crate `xylitol-official-plugin-dap`：`DapPluginConfig` + `DapPlugin`。
- [x] 2.2 工具 `dap_launch` / `dap_stack` / `dap_scopes` / `dap_variables` / `dap_evaluate` / `dap_continue` / `dap_disconnect`。
- [x] 2.3 懒启动 dapz `AgentHandle`；disconnect 释放。
- [x] 2.4 单测 + debugpy ignored roundtrip。

## 3. 配置与装配

- [x] 3.1 `AppConfig.lsp` / `AppConfig.dap`（Option，缺省零装配）。
- [x] 3.2 kernel `optional_plugin_rows` 追加 lsp/dap rows；`official_builtins` 注册插件。
- [x] 3.3 `just gen-config-schema` / `just gen-config-example`。

## 4. 规格与 BDD

- [x] 4.1 `infra-lsp` 规则与场景。
- [x] 4.2 `infra-dap` 规则与场景。
- [x] 4.3 BDD steps + bindings：零装配与工具注册。

## 5. 门禁与收尾

- [x] 5.1 `just qa` 全绿。
- [x] 5.2 `just qa-e2e` 全绿。
- [x] 5.3 `llman-sdd validate c3004-add-lsp-dap-tools --check --strict` 通过。
- [x] 5.4 verify 无 CRITICAL，finalize 归档；PR 用 rebase merge。
