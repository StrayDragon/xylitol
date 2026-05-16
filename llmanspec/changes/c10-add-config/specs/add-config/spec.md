---
llman_spec_valid_scope:
  - src/infra/config
llman_spec_valid_commands:
  - llman sdd validate c10-add-config --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo test -p xylitol --lib infra::config tests pass
---

```toon
kind: llman.sdd.delta
ops[3]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,yaml-parse,"System MUST parse YAML configuration from three layers (global/project/user) and deep-merge them with later layers overriding earlier ones.",null,null,null
  add_requirement,r2,schema-gen,"System MUST generate JSON Schema from Rust config types via schemars for IDE auto-completion.",null,null,null
  add_requirement,r3,runtime-validate,"System MUST validate merged config against JSON Schema at runtime and report human-readable errors.",null,null,null
op_scenarios[3]{req_id,id,given,when,then}:
  r1,happy,"a project config overrides model field","config is loaded","merged config contains project model value"
  r2,happy,"AppConfig struct is defined","schemars::schema_for is called","valid JSON Schema is generated to configs/config.schema.json"
  r3,happy,"a config file has invalid field","config is loaded","a descriptive error is returned with field path"
```
