# Tasks

## 1. 加载器旧键检测

- [x] 1.1 `plugins/xylitol-official-plugin-config/src/loader.rs`：新增 `reject_legacy_provider_preset(&merged)`，检测 `models.models.*.preset` 与顶层 `provider_presets`。
- [x] 1.2 错误文案含旧键路径、迁移方向与 `configs/example.yaml` 指针。
- [x] 1.3 在 serde 解析前调用，正常配置不受影响。

## 2. 单测

- [x] 2.1 model preset 被拒绝且文案含迁移指引。
- [x] 2.2 顶层 provider_presets 被拒绝。
- [x] 2.3 无旧键配置照常加载。

## 3. 规格与 BDD

- [x] 3.1 `runtime-config` 新增 `legacy-provider-preset-migration` 规则与场景。
- [x] 3.2 补 step 与 `#[scenario]` 绑定，`cargo test --all-features --test bdd` 全绿。

## 4. 文档

- [x] 4.1 README 增加 preset 回滚迁移说明（显式字段 / 网关）。

## 5. 门禁与收尾

- [x] 5.1 `just qa` 全绿。
- [x] 5.2 `just qa-e2e` 全绿。
- [x] 5.3 `llman-sdd validate c3000-update-config-legacy-migration --check --strict` 通过。
- [x] 5.4 verify 报告无 CRITICAL，finalize 归档。
