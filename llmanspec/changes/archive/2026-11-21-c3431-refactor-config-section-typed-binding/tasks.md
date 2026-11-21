# Tasks

## T1 sdk 绑定 trait

- [x] `config_section.rs` 新增 `SectionBinding`（`const KEY` + `schema_value` + `default_value` + `descriptor()` + `from_sections`）与 `SectionBindingError`（Missing/Parse 携带 KEY）。
- [x] 单测：descriptor 派生字段正确；from_sections 命中 / 缺键 / 解析失败三态。
- 验证：`cargo nextest run -p xylitol-plugin-sdk` 全绿。

## T2 声明方迁移（11 插件 + core）

- [x] 各插件类型化 config 结构体 impl `SectionBinding`；KEY 取原描述符字面量；`config_section()` 改为派生转发（或按 diff 最小直改调用点）。
- [x] `core_sections()` 同型迁移。
- [x] 每插件一条断言：`<T>::KEY` == 原字符串（防迁移改键）。
- 验证：`cargo nextest run -p xylitol-kernel` 全绿（含 load_with_sections 系）；`gen-config-schema` 输出 diff 为空（前后各跑一次比对）。

## T3 读取点 loud 化

- [x] 普查全部类型化读取点（rg `sections.get(` 与各插件类型结构体消费路径），逐个改 `from_sections`。
- [x] 对照测试：构造键漂移 / 类型错配置，断言显式报错而非静默默认。
- [x] 若普查发现静默回落被可观察场景依赖：STOP，改 `needs_specs_change: true` 并报告。（普查判定：预判成立，无需升级，理由见 T3 提交说明）
- 验证：`cargo nextest run` 相关 crate 全绿；对照测试红→绿地证明 loud 生效。

## T4 全量门禁

- [x] `just fmt` + `just lint`；`just qa` 全绿；`llman-sdd validate c3431-refactor-config-section-typed-binding --strict` failed 0。
- 验证：输出附于收口记录。
