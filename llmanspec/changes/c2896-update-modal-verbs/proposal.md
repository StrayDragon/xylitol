---
depends_on: []
---

## Why

#10：spec 规则描述与 GWT 步骤行沉积 RFC2119 情态动词（MUST/SHALL）。llman-sdd 自
native-gherkin-format 起约定规则描述为自由文本，情态词是旧约定遗产；其中步骤行被
runner 按文本匹配执行，情态词是纯噪声且放大步骤文本漂移面。纯措辞清理，行为合约
不变。

## What Changes

- 全部 46 个 spec 中 GWT 关键词（那么/当/并且/而且/假如/但是）开头的行：清零
  MUST/SHALL——真步骤行（6 空格缩进）删情态词（MUST NOT → 不），当开头的规则
  描述行改中文情态（MUST NOT → 不得，MUST/SHALL → 须）
- 同步 `tests/bdd/steps_*.rs` 中按文本匹配的 `#[given|when|then("…")]` 步骤定义
- 专项批清理 Top 8 密集文件全部规则描述行的 MUST/SHALL：server-core、
  app-tui-host、agent-tools、app-tui-transcript、app-tui-input、domain-compaction、
  agent-session-store、agent-runtime
- 其余文件描述行随触清（本批不动）；上游 llman-sdd 模板措辞修正不在本仓范围
