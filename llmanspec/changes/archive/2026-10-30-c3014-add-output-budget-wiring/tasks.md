# Tasks

## 1. SDK 配置与元数据
- [x] `XyModelEntryConfig` 新增 `max_output_tokens`，`to_model_meta` 写入 `max_tokens`
- [x] 0 值校验拒绝
- [x] 单测：声明值进入 meta；未声明为 0；0 被拒绝

## 2. 运行时接线
- [x] `prepare_turn_binding` 把非零 `max_tokens` 填入生成选项
- [x] 单测：0 → None，非零 → Some

## 3. 配置产物
- [x] 再生成 `configs/config.schema.json`
- [x] 示例配置模板增加 `max_output_tokens` 说明并再生成 `configs/example.yaml`

## 4. 适配器 body 断言
- [x] Responses：选项存在时 body 含 `max_output_tokens`
- [x] Completions：选项存在时 body 含 `max_completion_tokens`
- [x] Anthropic：沿用既有预算断言

## 5. 门禁
- [x] `cargo test -p xylitol-plugin-sdk -p xylitol-runtime -p xylitol-official-plugin-config` 绿
- [x] `just qa` 绿
