# Tasks

## 1. 配置与信任

- [x] 1.1 `ExternalPluginConfig` + `AppConfig.external_plugins`。
- [x] 1.2 `external_plugin_specs()`：trust / reserved / duplicate / empty 过滤与 warn。
- [x] 1.3 `RESERVED_PLUGIN_NAMES` + `is_reserved_plugin_name()`。

## 2. 协议与工具桥

- [x] 2.1 `ExternalClient`：spawn + newline-delimited JSON-RPC 2.0 stdio。
- [x] 2.2 `initialize` 握手：protocol_version 1、tools 能力、工具列表。
- [x] 2.3 `ExternalPlugin`（rutis Plugin）注册工具与卸载清理。
- [x] 2.4 `ExternalTool`（XyTool）转发 `tools/call`，超时终止。
- [x] 2.5 公开名 `{plugin}__{tool}` + provider-safe 校验 + 不覆盖同名。

## 3. 装配接线

- [x] 3.1 `boot_ctx` 动态注册 Builtins 并追加 loader rows。
- [x] 3.2 复用 Chain + Admit；核心 / 官方名不可覆盖。
- [x] 3.3 失败隔离：spawn / 握手失败 warn 且不阻断装配。

## 4. 配置产物与文档

- [x] 4.1 `just gen-config-schema` / `just gen-config-example`。
- [x] 4.2 demo README + lab probe。

## 5. Demo 与测试

- [x] 5.1 Bun/TS demo：echo / add。
- [x] 5.2 内核单测：Python fixture 握手 + 调用、协议不匹配、命名。
- [x] 5.3 boot 测：受信 / 未受信 / 保留名 / 失败命令。
- [x] 5.4 BDD `infra-plugin` 六场景。
- [x] 5.5 本地 Bun demo 验证（lab probe + ignored 单测）。

## 6. 门禁与收尾

- [x] 6.1 `just qa` 全绿。
- [x] 6.2 `just qa-e2e` 全绿。
- [x] 6.3 `llman-sdd validate c3003-add-external-plugins --check --strict` 通过。
- [x] 6.4 verify 无 CRITICAL，finalize 归档；PR 用 rebase merge。
