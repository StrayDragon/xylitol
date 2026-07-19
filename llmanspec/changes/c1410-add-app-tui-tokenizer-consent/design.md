# Design: c1410 TUI tokenizer consent（purpose-draft）

## 相对 M1 的边界

```text
M1 CLI（已落地）     xylitol tokenizer status|download|clean
M2 TUI（本 change）  会话内确认 → 委托同一 bridge 缓存 API
M3 Web（后置）       控制台管理页
```

TUI **禁止** `reqwest` / 自建缓存根；只调已有 Driver seam 或组合根暴露的 tokenizer 端口。

## 触发与状态机（草案）

```text
need_precise && mapped_hf && !cached && policy.ask
  → show consent (repo/file/hf_base/dest/size?)
      → accept → download_opt_in → refresh estimate provenance
      → skip|esc → mark skipped (粒度 TBD) → heuristic OK
```

Busy 时：不弹下载确认抢 abort Esc；仅 idle / 模型切换等安全窗口。

## UI

待钉开放问题：解冻窄域 Choice vs 新 overlay。**禁止**在 stub Plate/Settings 上扩活配置编辑。

## 测试

- harness：accept → cache hit；skip → 无网络、provenance 非 LocalTokenizer 伪装
- 回归：paa6 估计路径仍不 download
