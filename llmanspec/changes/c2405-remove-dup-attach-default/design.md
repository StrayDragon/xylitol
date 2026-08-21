# Design

## 裁决

- **canonical 归属**：attach 默认 URL 与 fail-closed 行为的 SSOT 定在 `cli-entry`——`ce19` 管 attach URL 解析规则（默认 `http://127.0.0.1:18790`、`--attach` > `--port`），`ce21` 管未在听 fail-closed。理由：入口行为（含旗标解析）本就归 CLI 表面 capability；`app-tui` 定位是跨切面索引，不应持有入口细节。
- **字面量断言迁移**：原 BDD step 持有的 `DEFAULT_ATTACH_URL == "http://127.0.0.1:18790"` 字符串钉死迁入 `src/app/core/attach.rs` 单测（与 `resolve_attach_url` 默认值断言同居）；BDD 层只保留行为级断言（ce21 场景）。
- **不合并项**：protocol-app（协议真源）/ server-core（Host 实现义务）/ app-tui-host（client 行为义务）的四象限条款主体不同，属分层各自陈述，保留。

## 迁移说明

| 删除物 | 替代覆盖 |
|---|---|
| app-tui `tui2` requirements 行 | cli-entry `ce19` + `ce21` |
| app-tui.feature `@req:tui2 attach-default` 场景 | cli-entry.feature `@req:ce21 tui-attach-host-down` + attach.rs 单测字面量钉 |
| tests/bdd `test_tui2_attach_default` binding + `w_tui2_start`/`t_tui2_attach_default` steps | 同上 |

零产品行为变化；纯合约去重。
