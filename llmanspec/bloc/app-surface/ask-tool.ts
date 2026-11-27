/**
 * TUI 澄清问卷状态机（Ask Tool）。
 *
 * 业务：产品 TUI 内置 ask 工具，仅 TUI 装配；Choice 槽挂载问卷；
 * 跳过视为成功；回答载荷回写；scrollback 人话摘要；远端审批队列；
 * 显式 review 标签页。
 *
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1167（tui-only-registration）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1168（choice-slot-mount）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1169（skip-success）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1170（answered-payload）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1171（scrollback-ask-rail）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1172（not-trust）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r3327（remote-approval-queue）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r3339（choice-review-tab-explicit）
 *
 * 实现落点：crates/xylitol-app-tui/src/（ask 工具与 Choice 槽）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type AskPhase = "registered" | "slotMounted" | "answered" | "skipped" | "cancelled" | "unregistered";

export interface AskContext {
  /** 是否运行在产品 TUI 面（仅 TUI 装配）。 */
  onTuiSurface: boolean;
  /** ask 工具是否已注册。 */
  registered: boolean;
  /** 问卷槽是否挂载。 */
  slotMounted: boolean;
  /** 是否打开 review 标签页。 */
  reviewTabOpen: boolean;
  /** 回答载荷。 */
  answers: string[];
  /** scrollback 摘要是否已写。 */
  scrollbackSummaryWritten: boolean;
  /** 远端审批队列长度。 */
  remoteQueueDepth: number;
}

export const initialContext: AskContext = {
  onTuiSurface: true,
  registered: false,
  slotMounted: false,
  reviewTabOpen: false,
  answers: [],
  scrollbackSummaryWritten: false,
  remoteQueueDepth: 0,
};

export const meta = defineMeta({
  name: "ask-tool",
  pkg: "app-surface",
  purpose: "TUI 问卷：仅 TUI 装配、槽挂载、跳过成功、载荷回写",
  anchors: [
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1167",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1168",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1169",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1170",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1171",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r1172",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r3327",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r3339",
  ],
  impl: ["crates/xylitol-app-tui/src/"],
});

export const setupMachine = setup({
  types: {
    context: {} as AskContext,
    events: {} as
      | { type: "assemble" }
      | { type: "ask"; questions: number }
      | { type: "answer"; payload: string[] }
      | { type: "skip" }
      | { type: "cancel" }
      | { type: "openReviewTab" }
      | { type: "enqueueRemote" }
      | { type: "dequeueRemote" }
  },
  guards: {
    onTui: ({ context }) => context.onTuiSurface === true,
    offTui: ({ context }) => context.onTuiSurface === false,
    hasQuestions: ({ event }) => (event as { questions: number }).questions > 0,
    noQuestions: ({ event }) => (event as { questions: number }).questions === 0,
  },
  actions: {
    register: assign({ registered: () => true }),
    mountSlot: assign({ slotMounted: () => true }),
    recordAnswers: assign({
      answers: ({ event }) => [...(event as { payload: string[] }).payload],
      scrollbackSummaryWritten: () => true,
    }),
    markSkipped: assign({ slotMounted: () => false, scrollbackSummaryWritten: () => true }),
    markCancelled: assign({ slotMounted: () => false }),
    openReview: assign({ reviewTabOpen: () => true }),
    pushRemote: assign({ remoteQueueDepth: ({ context }) => context.remoteQueueDepth + 1 }),
    popRemote: assign({ remoteQueueDepth: ({ context }) => Math.max(0, context.remoteQueueDepth - 1) }),
  },
});

export const machine = setupMachine.createMachine({
  id: "ask-tool",
  initial: "unregistered",
  context: initialContext,
  states: {
    unregistered: {
      on: {
        assemble: [
          { guard: "onTui", target: "registered", actions: "register" },
          { guard: "offTui" },
        ],
      },
    },
    registered: {
      on: {
        ask: [
          { guard: "hasQuestions", target: "slotMounted", actions: "mountSlot" },
          { guard: "noQuestions", target: "skipped", actions: "markSkipped" },
        ],
        enqueueRemote: { actions: "pushRemote" },
        dequeueRemote: { actions: "popRemote" },
      },
    },
    slotMounted: {
      on: {
        answer: { target: "answered", actions: "recordAnswers" },
        skip: { target: "skipped", actions: "markSkipped" },
        cancel: { target: "cancelled", actions: "markCancelled" },
        openReviewTab: { actions: "openReview" },
      },
    },
    answered: { type: "final" },
    skipped: { type: "final" },
    cancelled: { type: "final" },
  },
});

export const scenarioGroups = {
  "TUI 装配注册 ask": [
    { name: "装配", events: [{ type: "assemble" }], expectStates: ["registered"], expectContext: { registered: true } },
  ],
  "挂载 Choice 槽": [
    { name: "装配并发起问卷", events: [{ type: "assemble" }, { type: "ask", questions: 2 }], expectStates: ["slotMounted"], expectContext: { slotMounted: true } },
  ],
  "回答载荷回写": [
    { name: "装配并发起问卷", events: [{ type: "assemble" }, { type: "ask", questions: 1 }], expectStates: ["slotMounted"] },
    { name: "提交回答", events: [{ type: "answer", payload: ["选项 A"] }], expectStates: ["answered"], expectContext: { scrollbackSummaryWritten: true } },
  ],
  "跳过视为成功": [
    { name: "装配并发起问卷", events: [{ type: "assemble" }, { type: "ask", questions: 1 }], expectStates: ["slotMounted"] },
    { name: "跳过", events: [{ type: "skip" }], expectStates: ["skipped"], expectContext: { slotMounted: false } },
  ],
  "无问题直接跳过": [
    { name: "装配并发起空问卷", events: [{ type: "assemble" }, { type: "ask", questions: 0 }], expectStates: ["skipped"] },
  ],
  "取消问卷": [
    { name: "装配并发起问卷", events: [{ type: "assemble" }, { type: "ask", questions: 1 }], expectStates: ["slotMounted"] },
    { name: "取消", events: [{ type: "cancel" }], expectStates: ["cancelled"] },
  ],
  "review 标签页显式打开": [
    { name: "装配并发起问卷", events: [{ type: "assemble" }, { type: "ask", questions: 1 }], expectStates: ["slotMounted"], expectContext: { reviewTabOpen: false } },
    { name: "显式打开 review", events: [{ type: "openReviewTab" }], expectStates: ["slotMounted"], expectContext: { reviewTabOpen: true } },
  ],
  "远端审批入队": [
    { name: "装配并入队两条远端审批", events: [{ type: "assemble" }, { type: "enqueueRemote" }, { type: "enqueueRemote" }], expectStates: ["registered"], expectContext: { remoteQueueDepth: 2 } },
    { name: "消费一条", events: [{ type: "dequeueRemote" }], expectStates: ["registered"], expectContext: { remoteQueueDepth: 1 } },
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
