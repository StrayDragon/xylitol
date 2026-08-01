---
version: "alpha"
name: "chrome-toast"
description: "Ephemeral chrome toast line above status (shell notice; not ScrollNotice)."
tokens_from: "../DESIGN.md"
components:
  chrome-toast-line:
    textColor: "{colors.warning}"
    height: "1"
---

# Chrome toast（壳层通告）

> Token：`{colors.warning}` → [`../DESIGN.md`](../DESIGN.md)。
> 词表：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。
> Status：[`status.md`](./status.md)。静图：[`playground/`](./playground/) 槽 `chrome-toast`。

## MUST

1. **落点**：layout 壳、位于 **status / spinner 槽上方**恰好 **1 行**（队列条之下、status 之上）。**MUST NOT** 写入 `UiModel.entries` / 任何 `UiEntry`（含 `ScrollNotice`）。
2. **形态**：整行 `{colors.warning}`；可见文案 **MUST** 以字面前缀 `Error: ` 开头，后接 body（body 可无此前缀）。窄宽 **MUST** 单行截断（`…`），**MUST NOT** 折成第二行。
3. **寿命**：单槽；新通告 **MUST** 替换旧通告；显示后 **MUST** 在约定 TTL（默认 **4s**，约 3–5s）内经 `idle_tick` / `Tick` 自动清除。
4. **与 status / 下轮预告**：toast **MUST NOT** 占用 status lead 或右侧 next-turn cue；spinner 仍独占 `{colors.accent}`。
5. **首用例**：agent/bang busy 下 Resume switch/rename/delete 拒闸 body = `BUSY_SESSION_SWITCH_NOTICE`（无 `Error:`）；渲染层拼前缀。**MUST NOT** 再对该路径 `push_scroll_notice`。
6. **MUST NOT** 用壳层通告冒充对话正文、下轮预告或队列条。

## 非目标

- 多行栈 / 角区浮动 / 鼠标关闭
- 把全部 C 类诊断迁出 ScrollNotice
