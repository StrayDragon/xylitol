# Design：模型档案输入模态声明与诚实拒绝

## 数据模型

- 新增 `XyModality { Text, Image, Audio, Video }`，serde 小写。
- `XyModelEntryConfig` 新增 `input_modalities: Option<Vec<XyModality>>`。
- 缺省推断：`OpenAi | Anthropic => [Text, Image]`，`Fake => [Text]`。
- 校验：列表非空且必须包含 `Text`；重复项去重保序；未知值由 serde 拒绝。

## 注册表

- `ModelRegistry` 增加 `modalities: HashMap<String, Vec<XyModality>>`。
- `register(meta)` 按 provider 缺省登记；新增 `register_with_modalities(meta, modalities)` 供 bootstrap 使用。
- `supports_modality(id, modality) -> bool`；未知模型返回 false。

## 运行时闸

- ReAct 在 `project_outbound` 之后、`attempt_model_stream` 之前检查出站消息是否含 `AgentPart::Image`。
- 当前模型未声明 `image` 时：发 `XyEvent::error_msg("model <id> does not declare image input; set input_modalities ...")` 并结束本 run，不调用 provider。
- 其他模态暂不做运行时闸（无公共部件入口）。

## 测试

- plugin-sdk：解析 `input_modalities`；provider 缺省；空列表/缺 text 拒绝。
- runtime：registry `supports_modality`；出站图像检测 helper；无 image 声明时 gate 返回拒绝。
- specs：`runtime-model-registry` 档案声明；`agent-runtime` 图像闸；`runtime-config` 字段校验。

## 非目标

- 音频/视频编解码、附件入口、抽帧、大小上限。
- 自动探测 provider 能力。
- 为未声明模态的模型做自动降级或静默丢附件。
