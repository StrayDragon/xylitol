# 设计：配置 schema 产物与代码同源生成

## 1. 现状

- 运行时校验的唯一来源是 `schema_for!(AppConfig)`（config 插件 `validate` 模块），加载路径不读 `configs/config.schema.json`。
- `configs/config.schema.json` 是手维护的**文档/编辑器契约**产物，自 2026-10-23 起未再更新，已落后于 `AppConfig`（缺 `tool_batch` / `tui` / `otel` / `loop_example` / `session.max_turns`）。
- 仓库已有 `app_config_schema()` 返回 `serde_json::Value`，但没有稳定序列化出口，也没有漂移检测。

## 2. 生成器形态

**bin 放在 config 插件内**（`plugins/xylitol-official-plugin-config/src/bin/gen_config_schema.rs`），理由：

- schemars 的 `schema_for!(AppConfig)` 只有该 crate 能直接调（`AppConfig` 定义在此）；
- 不给根包 CLI 加隐藏子命令（`xylitol gen-config-schema` 会进用户面 `--help`，属产品噪音）；
- 不进 `scripts/`（维护脚本不进门禁，且此处不需要 Python）。

**输出稳定化**（关键，否则每次生成都会产生无意义 diff）：

1. `serde_json::to_string_pretty`（两空格缩进）后统一加末尾换行；
2. `$defs` 与 `properties` 的键序由 `BTreeMap` 归一——schemars 已按结构声明序输出 `properties`，但 `$defs` 顺序随 derive 遍历可能抖动，故生成前对 `$defs` 递归按键名排序；
3. 不写时间戳、不写仓库绝对路径。

## 3. 门禁形态

**config 插件内的单测**（`validate` 模块 `#[cfg(test)]` 内）：

```text
render_app_config_schema() == 仓库 configs/config.schema.json 内容
```

- 走 `just test`（`cargo nextest run --workspace --all-features`），已包含在 `just qa`，零新增接线。
- 与 `scripts/check_*.py` 分工：脚本门禁管**跨文件文本约定**（crate 图、分层、版本命名、token），schema 漂移是**同 crate 内的代码-产物一致性**，用单测更直接、更快。
- 失败信息直接给出「请跑 `just gen-config-schema`」，避免读者猜。

**为什么不写 Python 检查脚本**：需要 `cargo run` 才能拿到 schema，等于在 `qa-check-scripts` 里嵌一次 cargo 调用；单测已在同一轮编译里，成本更低。

## 4. 幂等与边界

- 生成幂等：同一 `AppConfig` 两次生成字节相同（键序归一 + 无时间戳）；
- 路径解析：bin 与单测均以 `CARGO_MANIFEST_DIR` 上溯两级定位仓库根，避免依赖 CWD；
- 产物缺失时单测直接失败并提示生成命令（不静默创建）；
- `configs/config.schema.json` 保持「生成产物」定位，头注释不加（JSON 无注释位），改由 `configs/example.yaml` 顶部与 `justfile` 注释指向生成入口。

## 5. 验收

- `runtime-config` 新增一条规则 + 两个可执行场景：产物与代码一致、产物覆盖当前全部顶层配置节；
- `just qa` 绿（含新单测）；
- 故意改产物可复现红（人工核对一次，不进门禁）。
