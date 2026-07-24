# Design: c1598

```text
YAML ModelEntry.api ──► bootstrap register(XyModelMeta)
                              │
                              ├─ config.api = entry.api
                              └─ meta.api   = entry.api.unwrap_or_default()
                              ▼
                     resolve_adapter_kind(config)
                              │
                     Some(api) → from_config_str
                     None      → AdapterKind::default_for(kind)
```

单测闸：`yaml_model_api_is_honored_in_registry_config`（显式 completions / responses / 省略）。
