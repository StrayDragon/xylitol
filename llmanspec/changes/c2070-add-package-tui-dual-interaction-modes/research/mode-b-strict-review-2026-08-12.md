# Mode B Strict Architecture Review (2026-08-12)

> Change: `c2070-add-package-tui-dual-interaction-modes`
> Scope: xylitol-tui Mode B stack on branch work. **Research only** — no production/spec edits.
> Design intent (human, 2026-08-12): product app **Mode B-only**; library → **two independent entries** (Inline vs ApplicationOwned), Pi-style Main/Alt; demo → two example files (drop `XYLITOL_AGENT_DEMO_MODE`).

## Executive summary

Mode B behavior (alt + viewport + transcript select + dock clamp + OSC52 + copy-notice + suspend restore + exit dump) is largely implemented and unit-covered under a **single `TUI` with `if application_session_active`**. That matches today’s `InteractionMode` seam and ptim01–15 *observables*, but **blocks** the Pi-like dual-entry goal: paint, mouse, teardown, and Component hooks are entangled on one type. Host seam (ptim14) is incomplete: Editor remapping / copy-notice paint live in `agent_demo`, not a reusable ApplicationOwned entry. Spec/docs still say product default Mode A (`PI_DELTAS` D16, live ptim14 wording) vs the new B-only product decision. Layer-5 PTY/tmux Mode B is **human-demo only** (`just demo-tui-alt-screen`). Highest leverage next: extract ApplicationOwned paint/session module, split demos, then formalize host Editor hit-test + dump opt-out before product migration.

## Findings table

| Sev | Area | Evidence | Recommendation |
|---|---|---|---|
| P0 | Strategic / dual-entry | `tui.rs` 343–352, 1006–1008, 1046–1047, 1090–1107, 1199–1231, 1459–1603: one `TUI` + scattered `application_session_active` / `interaction_mode` branches; `PI_DELTAS.md` D16 still「单 TUI + InteractionMode；产品默认 Mode A」 | Treat dual-entry as explicit follow-on change: stop growing Mode B inside Inline paths; freeze new `if application_session_active` except bugfixes |
| P0 | Host seam ptim14 | Demo owns remapping + notice paint: `agent_demo.rs` 1124–1137, 1589–1642, 1649–1656, 4705–4711; library `Editor::set_screen_origin` (`editor.rs` 424–435) unused by demo’s custom dock math | Publish a Mode B host checklist / thin helper (dock rows → editor origin → `handle_mouse_local`) so product does not fork demo glue |
| P1 | Spec dump ptim02 | `finish_inline` always dumps when Mode B (`tui.rs` 1087–1107); unit covers dump (`interaction_modes_test.rs` 362–383); **no** config to disable (spec MAY) | Add `set_mode_b_exit_dump(bool)` (default on) before product; rename teardown away from `finish_inline` for ApplicationOwned |
| P1 | Editor seam ptim13 | Unit OK (`editor.rs` 2536–2611); edge-scroll **removed** (2092–2094) — aligns human cut, not ptim04 (transcript-only). Demo remaps then `handle_input(Mouse)` with origin 0 — parallel to `set_screen_origin` subtract path (2060–2069) → double-subtract risk if both used | One remapping contract: either always set origin + absolute mouse, or always local-only API; document + test the chosen path |
| P1 | Naming / API smell | `finish_inline` does Mode B alt-leave + dump (`tui.rs` 1087–1107); `set_interaction_mode` docs say prefer rebuild (`416–419`) but flag still lives on shared TUI | Split `finish_main_screen` / `finish_application_owned` (or entry-local Drop); deprecate mode flag mutation on a live stack |
| P1 | Docs / AGENTS drift | `AGENTS.md` 81, 98 + D16: Mode A product default / env demo Mode B; contradicts 2026-08-12 B-only product intent | Update PI_DELTAS D16 + AGENTS after dual-entry decision lands; keep Inline as library lab, not product default |
| P2 | Fowler: Shotgun surgery | Mode B touches `do_render`, `dispatch_event`, `idle_tick`, `start_impl` mouse Moved filter, suspend, Component hints (`tui.rs` 129–140, 1357–1363) | Move ApplicationOwned session object that owns project/mouse/tick/clipboard flush; Inline TUI should not know dock projection |
| P2 | Fowler: Feature envy / incomplete abstraction | `ModeBRuntime` is solid (`mode_b.rs` 15–187) but lifecycle + paint gate stay on `TUI`; `ModeBRuntime` not crate-root re-export (`lib.rs` 81–86 only `COPY_NOTICE_TTL`) | Re-export or nest runtime behind `ApplicationOwnedTui`; hide `application_session_active` as private session state |
| P2 | Demo env split | `agent_demo_wants_mode_b` (`agent_demo.rs` 970–983); `just demo-tui-alt-screen` | Split `agent_demo_inline.rs` / `agent_demo_alt.rs` (or `*_main`/`*_alt`); delete `XYLITOL_AGENT_DEMO_MODE` |
| P2 | Spec fidelity soft gaps | ptim04 transcript edge: unit OK (`selection.rs` 878–919); ptim11: unit OK (`interaction_modes_test.rs` 221–238) but suspend may rebuild empty `ModeBRuntime` if `None` (`tui.rs` 1156–1160); ptim15 paint is host-only (OK per MAY) | Assert scroll/selection survive suspend; optional library dock-band notice painter later |
| P2 | Standards | Package boundary OK (no main-crate deps). Sync demo loop embeds Mode B host duties — conflicts「`TUI::start()` 仅 demo」+「产品 host 驱动」when product copies demo | Product path: host-driven `dispatch_event`/`try_render`/`idle_tick` only; do not copy `start()` Mode B auto-begin without documenting |

## Entanglement map (Inline vs B shared state)

```text
                    ┌─────────────────────────────────────┐
                    │              TUI<T>                  │
                    │  components / overlays / diff paint │
                    │  previous_lines, viewport_top, …    │
                    └───────────────┬─────────────────────┘
                                    │
         ┌──────────────────────────┼──────────────────────────┐
         │ Inline (Mode A)          │  ApplicationOwned (B)     │
         │ interaction_mode=Inline  │  + application_session_   │
         │ no mode_b                │    active + mode_b: Some  │
         │ differential + scrollback│  project_frame caps H     │
         │ mouse optional (lab)     │  alt + mouse on begin     │
         │ finish_inline: park+stop │  finish_inline: leave alt │
         │                          │    + dump + stop          │
         └──────────────────────────┴──────────────────────────┘
                                    │
              Shared Component trait hooks (always present):
                mode_b_dock_rows_hint · wants_pointer_motion ·
                take_pending_clipboard · Editor.selection / origin

Mode B-only types (already separable):
  ModeBRuntime · SelectionController · ScrollView (B usage) ·
  COPY_NOTICE_TTL · format_osc52

Demo-owned (not library seam yet):
  dock → editor-local remap · copy-notice UI · env mode split ·
  after_dispatch_hook Mode B glue
```

**Hotspots for dual-entry cut:** `do_render` projection gate; `dispatch_event` transcript-first mouse; `finish_inline` dual teardown; `with_terminal_suspended` Mode B re-enter; Component Mode B hints on the shared trait.

## Suggested refactor roadmap (ordered, minimal; do not implement here)

1. **Freeze** new Mode B logic behind env/`InteractionMode` branching; document product B-only + library dual-entry intent in change notes (specs later).
2. **Split demos**: `agent_demo` (Inline) + `agent_demo_alt` (ApplicationOwned); recipes without `XYLITOL_AGENT_DEMO_MODE`.
3. **Extract** `ApplicationOwnedSession` (or type alias wrapping TUI+ModeBRuntime): own `begin`/`end`, `project_frame`, mouse pre-dispatch, clipboard flush, copy-notice tick — leave Inline `TUI` free of those fields.
4. **Teardown API**: `finish_application_owned` (dump + leave alt) vs `finish_inline`; keep thin shim for one release if needed.
5. **Host Editor contract**: one remapping path + unit/integration test that product can copy; move notice arming pattern into documented host snippet (or small helper).
6. **Dump opt-out** API (ptim02 MAY) + suspend state retention assertion.
7. **PI_DELTAS D16 / AGENTS** rewrite: two entries like Pi Main/Alt; product default ApplicationOwned; Inline retained as library/lab.
8. **Product migrate** last: app host uses ApplicationOwned entry only; no env mouse for product Inline (already AGENTS rule).

## E2E automation candidates

Today Mode B acceptance is **human** via `just demo-tui-alt-screen` (`AGENTS.md` ~98; tasks checklist). Layer-5 e2e still targets Inline `agent_demo` (`tests/tui_e2e/`). Candidates:

| Behavior | Harness hint |
|---|---|
| Alt enter on start; leave on exit | PTY spawn alt example; assert CSI `?1049h` / `?1049l` in raw stream (or LoggingVT-style if e2e captures writes) |
| Exit dump on main scrollback | After quit, capture PTY scrollback / post-leave writes contain transcript needles (mirror `mode_b_finish_dumps_*`) |
| Wheel persists viewport (not emulator scroll) | Inject wheel; snapshot viewport lines stay off follow-end across frames |
| Transcript drag → OSC52 | Mouse Down/Drag/Up in transcript; assert `\x1b]52;c;` outside sync batch |
| Dock press does not start transcript select; falls to editor | Down in dock band; no transcript invert; editor selection / focus path |
| Dock-edge clamp while dragging | Drag into dock mid-select; selection remains; Up still copies |
| Editor multi-line select independent | Multi-line editor text; drag in dock editor band; OSC52 from editor buffer only |
| Copy-notice cue ~2s | After copy, viewport shows notice needle; after TTL+idle_tick, gone; not in ScrollNotice/transcript |
| Suspend/resume (Ctrl+G stub) | Stub editor path; post-resume still alt + mouse (extend existing editor stub e2e) |

Prefer **dedicated alt example binary** for e2e so Inline demos stay stable. Keep human checklist for feel (edge autoscroll rate, OSC52 terminal support).

## Open questions for human

1. Dual-entry shape: two types (`InlineTui` / `ApplicationOwnedTui`) vs shared core + two facades — preference?
2. Product cutover: hard-cut Mode B-only, or keep Inline behind dead/lab flag in app?
3. Dump: always on for product, or need opt-out in first product PR?
4. Editor remapping: standardize on `set_screen_origin` + absolute mouse, or library `hit_test_dock` helper?
5. Spec/docs: update live `package-tui-interaction-modes` (ptim14「产品仍可为 Mode A」) + D16 now, or only after dual-entry change?
6. E2E priority: which 2–3 Mode B behaviors must gate `qa-e2e` first?

---

*Identifiers English; analysis Chinese/English mix. Cite paths above for apply/verify follow-ups.*
