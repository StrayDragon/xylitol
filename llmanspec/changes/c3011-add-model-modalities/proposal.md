---
depends_on: []
title: 模型档案输入模态声明与诚实拒绝
needs_specs_change: true
---

# 模型档案输入模态声明与诚实拒绝（#17 M1）

## Why

- 当前模型档案没有“支持哪些输入模态”的声明；图像部件会被直接送进 provider，模型不支持时只能由上游报错或静默失败。
- #17 的 M2/M3 音视频暂缓（缺少公共附件入口），先交付 M1：**声明 + 不支持时明确拒绝**，为后续媒体管线留同一份档案。

## What Changes

- 模型配置新增 `input_modalities`（`text` / `image` / `audio` / `video`），缺省按 provider 推断：OpenAI / Anthropic 为 `text + image`，Fake 为 `text`。
- 模型注册表记录并暴露 `supports_modality`；配置校验拒绝空列表、缺少 `text` 与未知模态。
- ReAct 在调用 provider 前检查出站消息：若含图像部件而当前模型未声明 `image`，发出明确错误并结束本 run，不调用 provider。
- 音视频只做档案声明，不接编解码/附件管线。

## Capabilities

- `runtime-model-registry`（档案声明与注册表查询）
- `agent-runtime`（图像模态闸与诚实拒绝）
- `runtime-config`（配置字段校验）

## Impact

- **行为变化**：未声明 `image` 的模型收到图像部件时从“上游报错/静默”变为“本地明确拒绝”。
- **兼容**：OpenAI / Anthropic 缺省仍为 `text + image`，既有读图路径不变；Fake 缺省 `text`。
- **不承诺**：音频/视频可理解、媒体附件入口、编码/抽帧。

## Further Notes

- 与 #17 对齐：M1 优先，M2/M3 待模型支持与公共附件场景明确后再开。
