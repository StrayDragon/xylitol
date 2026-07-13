# Design — c626 playground lint

## Decision

采用 **c625 `design.md`「静图一致性：L1 + L2」** 为格式 SSOT，本 change 只落地闸门实现。

## L1 范围细化

- `cNNN` 噪音：**仅**扫 Next-wave 面板（`panel-models` / `panel-tree-power`）与 `MODELS`/`TREEP` 模板源；历史壳标题（如 c475）不挡。
- 选中对比度：要求 CSS `.rev * { color: inherit }`；fixture 断言选中段无 `fg-user` 等。

## L2

- 夹具目录：`src/app/tui/design/fixtures/`
- HTML：`data-design-fixture="<id>"`
- 脚本解析 `source:`（`treep.filter` / `models.open` → JS 模板）

## 非目标

截图、与 Rust 逐字符一致。
