# Tasks

## 1. sdk 词表（加）
- [ ] xylitol-plugin-sdk 新增 config_section 模块：ConfigSection { key, schema: fn() -> serde_json::Value, default: fn() -> serde_json::Value }（不得带无消费者的 domain 列）
- [ ] 词表单测：key 判等、函数指针调用返回 Value

## 2. config 参数化（加）
- [ ] loader / validate / render_app_config_schema 接受节清单参数；现 sections() 静表态改组为 core_sections()（核心域节 + context_inject / otel / compaction / codemode 过渡节）
- [ ] ConfigPlugin 构造接受注入清单并贯通到加载与校验

## 3. 七插件自声明（加）
- [ ] hooks / loop-policies / todo / workflow / mcp / lsp / dap 各暴露 config_section()；schema / default 函数从 config sections.rs 搬迁，行为等值
- [ ] 每插件单测：default() 与旧表产物逐字节一致（对照在 change 分支上测）

## 4. kernel 聚合与接线
- [ ] boot 组装全量清单（core_sections + 7 声明）注入 ConfigPlugin；未知键校验改查注入清单
- [ ] lsp_dap_rows 等 row 组装改直连插件类型，删 config::types 中转引用
- [ ] gen-config-schema bin 迁至 kernel；justfile 改指向；产物验证：change 分支上再生成后 git diff configs/config.schema.json 为空

## 5. 拆旧（删）
- [ ] config 删 7 个插件 re-export（types.rs:671-674、816-818）与 Cargo 依赖；旧 SECTIONS 表删除
- [ ] rg 验证无 config::types 下七插件类型残留引用；cargo build 全绿

## 6. 验证
- [ ] cargo nextest run -p xylitol-plugin-sdk -p xylitol-kernel -p xylitol-official-plugin-config 与 7 插件包
- [ ] cargo nextest run --all-features --test bdd（runtime-config 场景绿）
- [ ] just qa
