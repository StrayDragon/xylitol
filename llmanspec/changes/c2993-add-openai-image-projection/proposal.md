---
depends_on: []
title: 让 OpenAI 族请求投影 AgentPart::Image
needs_specs_change: true
---

# 让 OpenAI 族请求投影 AgentPart::Image

## Why

- `Read` 工具已按 `agent-tools` r-read-image-parts 产出 `AgentPart::Image`，Anthropic 方言也会映射为 image block。
- OpenAI Responses / Completions 方言当前只走 `collect_text_parts` / `collect_user_text_parts`，图片被静默丢弃；模型收不到图，却看不到任何错误。
- 这是 c2943 附录里已核实的缺口；本 change 只取该增强面的 A 切片，不碰 ResourceLink / 帧上限 / 视频。

## What Changes

- 新增共享 helper `image_data_url`：`url` 优先，否则 `data:<media_type>;base64,<data>`。
- OpenAI Responses：
  - user message 的 content 数组写入 `input_image`；
  - tool result 的 `function_call_output.output` 在含图片时改为 `input_text` + `input_image` 数组。
- OpenAI Completions：
  - user message 含图片时改用 content 数组 + `image_url`；
  - tool message 只放文本；同一轮工具结果中的图片批量补发为紧邻的 user message（标注来源 call id），不把图片塞进 `role=tool`。
- 仅图片无文本时保留短占位文本。
- 更新 `infra-provider` 规则 `r2975` 与场景；插件单测与 BDD 覆盖。

## Capabilities

- `infra-provider`：OpenAI 族多模态图片投影。

## Impact

- `plugins/xylitol-official-plugin-provider-openai/src/provider/native/{mod,openai,openai_responses}.rs`
- `llmanspec/specs/infra-provider/infra-provider.feature`
- `tests/bdd/steps_bridge.rs` + `bindings_c2827.rs`
- `llmanspec/changes/c2943-add-resourcelink-agent/`：保持 park，仅在本 change 的 design 记录 A 已拆出。

## Out of Scope

- 不改 ResourceLink / `MAX_BODY_BYTES` / batch 上限。
- 不实现读视频。
- 不改 Anthropic 既有 image block 映射。
- 不改 `agent-tools` 的 Read 产出形状。
