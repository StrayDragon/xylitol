# Tasks — c1155-add-app-tui-paste-image

## 1. Infra / tool / domain helpers

- [x] 1.1 clipboard：temp 写出 helper（c7）
- [x] 1.2 image：`path → ImageContent/AgentPart`（i5，走 resize）
- [x] 1.3 `XyTool::execute_as_parts` + react 写入 parts；ReadTool 图片回 Image（t21）

## 2. Driver + 产品 TUI

- [x] 2.1 Driver 缝（读剪贴板图 + 写 temp）+ InProcess / Scripted / Remote stub
- [x] 2.2 host：`app.paste.image` → stage → editor 插绝对路径；失败短 Error
- [x] 2.3 提交路径保持纯文本（无 user Image 转码）

## 3. 测与校验

- [x] 3.1 harness：粘贴成功插路径 / 无图失败；read 单测含 Image part
- [x] 3.2 `llman sdd validate c1155… --strict --no-interactive`
- [x] 3.3 `just lint` + 相关测
