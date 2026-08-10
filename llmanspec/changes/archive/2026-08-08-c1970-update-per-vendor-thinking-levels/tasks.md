# Tasks：c1970-update-per-vendor-thinking-levels

## 0. 调研门禁

- [x] 0.1 写 `research/thinking-levels-per-vendor-2026.md`（DeepSeek / OpenAI / Anthropic 旋钮 + resume 两层）
- [x] 0.2 design 吸收拍板：仅 off 默认、配置声明字符串、resume 无损、末项换模默认

## 1. 合约落地（Specs landing）

- [x] 1.1 更新 `runtime-config`：`rc16`/`rc17` — 声明列表为 SSOT；取消「未知枚举名必失败」；map 键相对声明列表
- [x] 1.2 更新 `runtime-model-registry`：`m9`/`m10`/`m11` — 无 STANDARD；换模末项；resume/sticky；字符串进 generate
- [x] 1.3 更新相关 `.feature`（config / model-registry / session resume）对齐新语义
- [x] 1.4 更新 `app-tui-commands`（及必要 chrome）：UI 展示声明档，去掉「禁止厂商档名」义务

## 2. 实现切片

- [x] 2.1 配置解析与 meta：字符串列表 / map 校验；缺省 → `[off]`
- [x] 2.2 ModelManager / Driver：set 拒绝、换模末项、cycle、sticky resume、无条目默认 `off`
- [x] 2.3 Session load 路径：原样还原；禁止 clamp 写回 JSONL
- [x] 2.4 Bridge：自由串缺 map 的可观测失败；DeepSeek dialect 回归单测
- [x] 2.5 TUI `/model` 槽与 footer 跟随声明列表

## 3. 验证

- [x] 3.1 相关 BDD + `just test` 目标切片绿
- [x] 3.2 `just qa`（或约定门禁）绿
- [x] 3.3 文档：架构/示例 YAML 补 DeepSeek `[off, high, max]` 与 breaking 说明
