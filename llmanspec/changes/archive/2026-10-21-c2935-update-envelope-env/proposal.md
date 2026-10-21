---
depends_on: []
title: 信封格式开关环境变量改为 XYLITOL_ENVELOPE_V3
needs_specs_change: false
branch: sdd/c2935-update-envelope-env
base_branch: main
base_sha: 0e498b2cc444bff0aed1559c894d48deb3df195d
---

# 信封格式开关环境变量改为 XYLITOL_ENVELOPE_V3

## Why

用户可设名仍叫 `XYLITOL_WIRE_V3`，与代码标识 `ENVELOPE_FORMAT_ENV` 以及产品「信封」叙事不一致。本批 crate 化收口要求对齐命名，且仓库改名纪律不留旧名别名。

## What Changes

- 用户可设环境变量改为 `XYLITOL_ENVELOPE_V3`。
- 未设时产品默认仍开。
- 旧名 `XYLITOL_WIRE_V3` 不再读取。
- content-type `application/x-fory-v3` 与 fbs `namespace xy.wire.v3` 不动。

## Capabilities

- `protocol-app`（r1940）

## Impact

设置了旧名的用户必须改用新名；未设环境变量的产品路径不变。
