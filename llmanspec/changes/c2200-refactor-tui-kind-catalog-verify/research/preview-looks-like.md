# 一条 Preview 在代码里长什么样

> 给「还差点，先看伪代码」用。这不是要合入的 API；是把
> `preview-by-construction.md` 落到 **现有类型旁边** 的形状，方便拍板。

## 0. 今天已经有 80%

| 已有 | 缺的 |
|---|---|
| `DebugSceneMeta { id, description }` | stability / covers / 语义不变量 |
| `LiveWindowFrame { events, must, must_not }` | 子串碰巧命中；没有 L2/L1 |
| `/debug activity-fold-live` 真 `HostSession` paint | 与 catalog 表是两路（live 还被 `resolve_scene_id` 排除） |
| `ActivityCounts` + `format_cluster_body` | 测里很少 dump 给人看 |

目标：**把 live_tape 升成 Preview 表的一行**，不是新 Scene 框架。

## 1. 类型（扩 catalog，不新 crate）

```rust
// 建议落点：src/app/debug_fixtures/preview.rs
// 产品 HostSession 已有；TestTerminal 已有。不要 xylitol-tui 再包一层 Widgetbook。

pub enum PreviewStability {
    /// 目录 + 按键 + 语义 invariants。不挡合并，不进视觉 insta。
    Evolving,
    /// 合入要接受：去 spinner 的屏 + 语义 dump。改外观 = 改基线。
    Frozen,
}

/// Inventory：新 UiEntry 变体 / chrome 槽编译期或测里必须能被点到。
pub enum PreviewCovers {
    UiEntryThinking,
    UiEntryTool,
    UiEntryAsk,
    UiEntryTodo,
    UiEntryCompaction,
    UiEntryBash,
    ChromeFooter,
    ChromeEditor,
    // … 与 UiEntry / chrome 槽 1:1，禁止 `_ =>` 漏登记
}

pub struct Preview {
    pub id: &'static str,                 // `/debug <id>` 与测共用
    pub description: &'static str,        // 人读一行
    pub covers: PreviewCovers,
    pub stability: PreviewStability,
    /// 把已经 `new_product_ui` 的 session 推到该态。
    pub mount: fn(&mut HostSession<TestTerminal>),
    /// 可选：看交互（折叠加弦、Choice）。空 = 只看静态态。
    pub keys: &'static [Key],
    /// 语义树，不是 `plain.contains("Thought")`。
    pub invariants: fn(&PreviewView) -> Result<(), String>,
}

pub struct PreviewView<'a> {
    pub plain: String,           // 去 ANSI 的产品 render
    pub dump: String,            // L3/L2/L1 语义树（人+测）
    pub session: &'a HostSession<TestTerminal>,
}
```

`DEBUG_SCENES` 变成 `PREVIEWS` 的投影（id + description），`/debug` 不另维护第二张表。

## 2. 完整一例：thought-only（Evolving）

今天 live_tape 帧 2 只保证屏上出现 `"Thinking"`。Preview 要保证 **和弦 + 计数角色**。

```rust
fn mount_thought_only(session: &mut HostSession<TestTerminal>) {
    let mut model = session.ui_model().clone();
    model.begin_run("preview: thought-only");
    apply_xy_event(&mut model, &XyEvent::ThinkingDelta("consider next edit".into()));
    apply_xy_event(&mut model, &XyEvent::AgentEnd { messages: vec![] });
    *session.ui_model_mut() = model;
    session.sync_ui_root_from_model();
    let _ = session.render_now();
}

fn invariants_thought_only(v: &PreviewView<'_>) -> Result<(), String> {
    // dump 例（实现落 activity_fold，不是手搓第二套 scrollback）:
    //   L3 envelope  Worked for 1s
    //   L2 cluster   thought-only  Thought 1s
    //   L1 block     Thinking id=t1 sealed
    if !v.dump.contains("L2 thought-only") {
        return Err(format!("expected thought-only cluster\n{}", v.dump));
    }
    if v.dump.contains("Used") {
        return Err(format!("todo/unknown must not leak into thought-only\n{}", v.dump));
    }
    if v.plain.contains("Planning next moves") {
        return Err("thought-only must not show Planning".into());
    }
    Ok(())
}

pub const PREVIEW_THOUGHT_ONLY: Preview = Preview {
    id: "activity-fold.thought-only",
    description: "Sealed thinking, no tools — Thought header, no Used",
    covers: PreviewCovers::UiEntryThinking,
    stability: PreviewStability::Evolving, // 词表还在抖：先能看见+断言，不锁像素
    mount: mount_thought_only,
    keys: &[], // 以后加折叠键：&[Key::Char('j')]
    invariants: invariants_thought_only,
};
```

人怎么看见：产品里 `/debug activity-fold.thought-only`（与现在 live 一样走 `HostSession`）。

测怎么锁语义（所有 Evolving 共用这一段，不是每个场景复制 harness）：

```rust
#[test]
fn preview_invariants() {
    for p in PREVIEWS {
        let mut s = HostSession::new_product_ui(TestTerminal::new(100, 32));
        (p.mount)(&mut s);
        for k in p.keys {
            s.handle_key(*k);
        }
        let view = PreviewView {
            plain: strip_ansi(&s.render_plain()),
            dump: semantic_dump(&s), // 现成 ActivityFoldState + counts
            session: &s,
        };
        (p.invariants)(&view).unwrap_or_else(|e| panic!("{}: {e}", p.id));
    }
}
```

## 3. Frozen 只多两行

把上面那条改 `stability: Frozen` 之后，**同一 mount** 额外：

```rust
#[test]
fn preview_frozen_baselines() {
    for p in PREVIEWS.iter().filter(|p| matches!(p.stability, Frozen)) {
        let mut s = HostSession::new_product_ui(TestTerminal::new(100, 32));
        (p.mount)(&mut s);
        let plain = strip_spinner(strip_ansi(&s.render_plain()));
        insta::assert_snapshot!(p.id, plain);
        insta::assert_snapshot!(format!("{}-dump", p.id), semantic_dump(&s));
    }
}
```

动效：不要 Frozen 黄金帧对着 spinner。帧带继续用 `LiveWindowFrame` 那种 **MockClock + named checkpoints**（已经存在）。Frozen 拍 **静止态**。

## 4. Inventory（新形态忘登记会红）

```rust
#[test]
fn every_ui_entry_kind_has_a_preview() {
    for covers in PreviewCovers::ALL {  // 穷举，禁止 _
        assert!(
            PREVIEWS.iter().any(|p| p.covers == covers),
            "add an Evolving Preview that covers {covers:?}"
        );
    }
}
```

ActivityAtom（穷尽 `UiEntry` → 计数角色）管 **不会静默吞块**。
Preview inventory 管 **新块至少能被看见、能按键**。两闸互补，不是重复。

## 5. 刻意不写的（看起来更「框架」、会再造 SSOT）

```rust
// 不要：运行时插件
inventory::submit! { Preview { ... } }

// 不要：YAML 第二实现（LSP 跳不进 mount，种子和产品类型漂移）
// previews/thought-only.yaml

// 不要：agent_demo / HTML playground 当 mount
session.paint_demo_string("Thought 1s");
```

种子数据可以是 Rust 函数（如现 `seed.rs`）或 `XyEvent` tape（如现 `live_tape.rs`）。**mount 必须以 `apply_ui_model` / `sync_ui_root_from_model` 结尾。**

## 6. 和现状的迁移（小）

| 现在 | 变成 |
|---|---|
| `DEBUG_SCENES` 7 条 | 各变一条 Preview；`activity-fold-live` **收进表**（今天被 `resolve_scene_id` 排除是历史债） |
| `LiveWindowFrame.must` 子串 | 保留作帧带；静止态改走 `invariants` + dump |
| `design/activity-fold.md` MUST | 意图注释；可观察句搬到对应 Preview 的 `invariants` |
| 手写 `playground/index.html` | 不删也可以，但 **禁止** 当完成条件 |

新组件完成条件（拍板用一句话）：

> 至少一条 `Evolving` Preview：`HostSession::new_product_ui` → `mount` → 可 `/debug` 打开 → `invariants` 绿。要锁 UI 再标 `Frozen`。
