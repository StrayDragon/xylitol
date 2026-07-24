# Design: c1600

```text
省略 api
  YAML ModelEntry     → AdapterKind::default_for(kind)   (已有 / c1598)
  JSON ModelManifest  → 同上（本 change 修 default_api 写死 Completions）

观测
  llm.request.api              （已有）
  agent.turn.xylitol.model.api （本 change）
```

文档：`docs/architecture/多厂商模型.md`、`配置与档案.md`、`configs/example.yaml`。
