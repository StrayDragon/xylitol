/**
 * TUI 输入键位路由状态机（Input Key Routing）。
 *
 * 业务：Esc / Ctrl+C 的取消与退出语义、忙碌时 Enter 作 steer、Alt+Enter 作 follow-up、
 * 空编辑器双 Esc 打开会话树、overlay 时 Ctrl+C 先关槽不 abort。
 *
 * @spec llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1294（cancel-and-ctrl-c）
 * @spec llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1305（steer-and-followup-keys）
 * @spec llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1321（double-esc-session-tree）
 * @spec llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1283（editor-operation-zone）
 * @spec llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1293（bang-esc-aborts）
 *
 * 实现落点：crates/xylitol-app-tui/src/（input 键位接线）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type InputPhase = "idle" | "busy" | "bangBusy" | "overlayOpen" | "exited";

export interface InputContext {
  /** 编辑器草稿。 */
  draft: string;
  /** 当前是否 busy。 */
  busy: boolean;
  /** 是否 bang 忙碌。 */
  bangBusy: boolean;
  /** 是否打开 overlay。 */
  overlay: boolean;
  /** abort 调用次数。 */
  aborts: number;
  /** steer 入队次数。 */
  steers: number;
  /** follow_up 入队次数。 */
  followUps: number;
  /** 是否请求退出。 */
  exitRequested: boolean;
  /** 会话树是否打开。 */
  treeOpen: boolean;
  /** 最近一次 Esc 时刻（用于双 Esc 时间窗）。 */
  lastEscAt: number | null;
}

export const initialContext: InputContext = {
  draft: "",
  busy: false,
  bangBusy: false,
  overlay: false,
  aborts: 0,
  steers: 0,
  followUps: 0,
  exitRequested: false,
  treeOpen: false,
  lastEscAt: null,
};

export const meta = defineMeta({
  name: "tui-input-routing",
  pkg: "app-surface",
  purpose: "键位语义：Esc/Ctrl+C 取消与退出、steer/follow-up、双 Esc 会话树",
  anchors: [
    "llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1294",
    "llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1305",
    "llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1321",
    "llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1283",
    "llmanspec/specs/app-tui-input/app-tui-input.feature#@req:r1293",
  ],
  impl: ["crates/xylitol-app-tui/src/"],
});

export const setupMachine = setup({
  types: {
    context: {} as InputContext,
    events: {} as
      | { type: "beginBusy" }
      | { type: "beginBangBusy" }
      | { type: "endBusy" }
      | { type: "openOverlay" }
      | { type: "type"; text: string }
      | { type: "esc"; at: number }
      | { type: "ctrlC"; at: number }
      | { type: "enter" }
      | { type: "altEnter" }
  },
  guards: {
    isBusy: ({ context }) => context.busy === true,
    isIdle: ({ context }) => context.busy === false,
    overlayOpen: ({ context }) => context.overlay === true,
    overlayClosed: ({ context }) => context.overlay === false,
    draftNonEmpty: ({ context }) => context.draft.length > 0,
    draftEmpty: ({ context }) => context.draft.length === 0,
    treeClosed: ({ context }) => context.treeOpen === false,
    treeOpen: ({ context }) => context.treeOpen === true,
    draftEmptyAndInWindow: ({ context, event }) => {
      const at = (event as { at: number }).at;
      return context.draft.length === 0 && context.treeOpen === false && context.lastEscAt !== null && at - context.lastEscAt <= 400;
    },
    withinDoubleEscWindow: ({ context, event }) => {
      const at = (event as { at: number }).at;
      return context.lastEscAt !== null && at - context.lastEscAt <= 400;
    },
    outsideDoubleEscWindow: ({ context, event }) => {
      const at = (event as { at: number }).at;
      return context.lastEscAt === null || at - context.lastEscAt > 400;
    },
  },
  actions: {
    markBusy: assign({ busy: () => true }),
    markBangBusy: assign({ busy: () => true, bangBusy: () => true }),
    clearBusy: assign({ busy: () => false, bangBusy: () => false }),
    markOverlay: assign({ overlay: () => true }),
    closeOverlay: assign({ overlay: () => false }),
    appendDraft: assign({ draft: ({ context, event }) => context.draft + (event as { text: string }).text }),
    clearDraft: assign({ draft: () => "" }),
    countAbort: assign({ aborts: ({ context }) => context.aborts + 1 }),
    requestExit: assign({ exitRequested: () => true }),
    enqueueSteer: assign({ steers: ({ context }) => context.steers + 1, draft: () => "" }),
    enqueueFollowUp: assign({ followUps: ({ context }) => context.followUps + 1, draft: () => "" }),
    stampEsc: assign({ lastEscAt: ({ event }) => (event as { at: number }).at }),
    openTree: assign({ treeOpen: () => true, lastEscAt: () => null }),
    closeTree: assign({ treeOpen: () => false, lastEscAt: () => null }),
  },
});

export const machine = setupMachine.createMachine({
  id: "tui-input-routing",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        beginBusy: { target: "busy", actions: "markBusy" },
        beginBangBusy: { target: "bangBusy", actions: "markBangBusy" },
        openOverlay: { target: "overlayOpen", actions: "markOverlay" },
        type: { actions: "appendDraft" },
        esc: [
          { guard: "treeOpen", actions: "closeTree" },
          { guard: "draftEmptyAndInWindow", actions: "openTree" },
          { guard: "draftEmpty", target: "idle", actions: "stampEsc" },
          { guard: "draftNonEmpty", actions: "clearDraft" },
        ],
        ctrlC: [
          { guard: "draftNonEmpty", actions: "clearDraft" },
          { guard: "draftEmpty", target: "exited", actions: "requestExit" },
        ],
        enter: { actions: "clearDraft" },
      },
    },
    busy: {
      on: {
        esc: { actions: "countAbort" },
        ctrlC: { actions: "countAbort" },
        type: { actions: "appendDraft" },
        enter: { actions: "enqueueSteer" },
        altEnter: { actions: "enqueueFollowUp" },
        openOverlay: { target: "overlayOpen", actions: "markOverlay" },
        endBusy: { target: "idle", actions: "clearBusy" },
      },
    },
    bangBusy: {
      on: {
        esc: { actions: "countAbort" },
        ctrlC: { actions: "countAbort" },
        endBusy: { target: "idle", actions: "clearBusy" },
      },
    },
    overlayOpen: {
      on: {
        ctrlC: { target: "busy", actions: "closeOverlay" },
        esc: { target: "busy", actions: ["closeOverlay", "closeTree"] },
      },
    },
    exited: { type: "final" },
  },
});

export const scenarioGroups = {
  "忙碌Esc中止不退出": [
    { name: "进入忙碌", events: [{ type: "beginBusy" }], expectStates: ["busy"] },
    { name: "按下 Esc", events: [{ type: "esc", at: 100 }], expectStates: ["busy"], expectContext: { aborts: 1, exitRequested: false } },
  ],
  "忙碌CtrlC中止同语义": [
    { name: "进入忙碌", events: [{ type: "beginBusy" }], expectStates: ["busy"] },
    { name: "按下 Ctrl+C", events: [{ type: "ctrlC", at: 100 }], expectStates: ["busy"], expectContext: { aborts: 1, exitRequested: false } },
  ],
  "busy回车改向Alt回车追问": [
    { name: "进入忙碌并输入 nudge", events: [{ type: "beginBusy" }, { type: "type", text: "nudge" }], expectStates: ["busy"] },
    { name: "Enter 作 steer", events: [{ type: "enter" }], expectStates: ["busy"], expectContext: { steers: 1, followUps: 0 } },
    { name: "输入 later 并按 Alt+Enter", events: [{ type: "type", text: "later" }, { type: "altEnter" }], expectStates: ["busy"], expectContext: { steers: 1, followUps: 1 } },
  ],
  "overlay时CtrlC先关槽": [
    { name: "忙碌并打开 overlay", events: [{ type: "beginBusy" }, { type: "openOverlay" }], expectStates: ["overlayOpen"] },
    { name: "Ctrl+C 关槽且不 abort", events: [{ type: "ctrlC", at: 100 }], expectStates: ["busy"], expectContext: { aborts: 0, overlay: false } },
  ],
  "空闲有草稿CtrlC清空": [
    { name: "输入草稿", events: [{ type: "type", text: "草稿" }], expectStates: ["idle"] },
    { name: "Ctrl+C 清空不退出", events: [{ type: "ctrlC", at: 100 }], expectStates: ["idle"], expectContext: { exitRequested: false, aborts: 0 } },
  ],
  "空闲空编辑器CtrlC退出": [
    { name: "Ctrl+C 请求退出", events: [{ type: "ctrlC", at: 100 }], expectStates: ["exited"], expectContext: { exitRequested: true } },
  ],
  "双Esc打开会话树": [
    { name: "第一次 Esc", events: [{ type: "esc", at: 100 }], expectStates: ["idle"], expectContext: { lastEscAt: 100 } },
    { name: "窗口内第二次 Esc 打开树", events: [{ type: "esc", at: 300 }], expectStates: ["idle"], expectContext: { treeOpen: true } },
  ],
  "树打开Esc恢复编辑器": [
    { name: "双 Esc 打开树", events: [{ type: "esc", at: 100 }, { type: "esc", at: 300 }], expectContext: { treeOpen: true } },
    { name: "Esc 关闭树", events: [{ type: "esc", at: 500 }], expectStates: ["idle"], expectContext: { treeOpen: false } },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

if (import.meta.main) {
  const results = runScenarioGroups(machine, scenarioGroups);
  const failed = Object.entries(results).filter(([, r]) => !r.passed);
  if (failed.length > 0) {
    for (const [group, r] of failed) {
      console.error(`组 ${group}`);
      for (const line of r.failures) console.error(`  ${line}`);
    }
    process.exit(1);
  }
  console.log(`${meta.name}: ${Object.keys(results).length} 组场景通过`);
}