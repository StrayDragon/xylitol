/**
 * 回合协调器状态机（Turn Coordinator）。
 *
 * 业务：一个 turn 从空闲到起始、流式、取消，最后收尾；同一时刻至多一个 turn 在写会话。
 * 这是 crates/xylitol-runtime/src/agent/runtime/state.rs 中 RuntimePhase 的可执行描述。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3322（one-submission-one-turn）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3375（会话信箱单写者）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1028（abort 队列语义）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1030（abort 取消 provider 流）
 * @spec llmanspec/specs/layer-architecture/layer-architecture.feature#@req:r1514（一 session 一写者）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/runtime/state.rs（RuntimePhase / TurnEvent / TurnTransition）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 协调器相位，对应 Rust 的 RuntimePhase。 */
export type CoordinatorPhase = "idle" | "starting" | "streaming" | "cancelling";

export interface CoordinatorContext {
  /** 当前 turn 序号；每次 Begin 递增。 */
  turnSeq: number;
  /** 进行中的 turn id；idle 时为 null。 */
  activeTurnId: number | null;
  /** 被拒绝的 Begin 次数（Busy）。 */
  rejectedBegins: number;
  /** 取消信号是否已下发。 */
  cancelRequested: boolean;
  /** 已完成的 turn 数。 */
  finishedTurns: number;
  /** 最近一次收尾原因。 */
  lastFinishReason: string | null;
}

export const initialContext: CoordinatorContext = {
  turnSeq: 0,
  activeTurnId: null,
  rejectedBegins: 0,
  cancelRequested: false,
  finishedTurns: 0,
  lastFinishReason: null,
};

export const meta = defineMeta({
  name: "turn-coordinator",
  pkg: "core-runtime",
  purpose: "单 turn 相位机：空闲→起始→流式→收尾，含忙拒绝与取消",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3322",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3375",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1028",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1030",
    "llmanspec/specs/layer-architecture/layer-architecture.feature#@req:r1514",
  ],
  impl: ["crates/xylitol-runtime/src/agent/runtime/state.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as CoordinatorContext,
    events: {} as
      | { type: "Begin" }
      | { type: "StreamStarted" }
      | { type: "Cancel" }
      | { type: "Finish"; reason: "done" | "aborted" | "error" }
  },
  actions: {
    openTurn: assign({
      turnSeq: ({ context }) => context.turnSeq + 1,
      activeTurnId: ({ context }) => context.turnSeq + 1,
      cancelRequested: () => false,
    }),
    rejectBegin: assign({
      rejectedBegins: ({ context }) => context.rejectedBegins + 1,
    }),
    requestCancel: assign({
      cancelRequested: () => true,
    }),
    closeTurn: assign({
      activeTurnId: () => null,
      finishedTurns: ({ context }) => context.finishedTurns + 1,
      lastFinishReason: ({ event }) => (event as { reason: string }).reason,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "turn-coordinator",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        Begin: { target: "starting", actions: "openTurn" },
        Cancel: { actions: "requestCancel" },
        Finish: { actions: "closeTurn" },
      },
    },
    starting: {
      on: {
        Begin: { actions: "rejectBegin" },
        StreamStarted: { target: "streaming" },
        Cancel: { target: "cancelling", actions: "requestCancel" },
        Finish: { target: "idle", actions: "closeTurn" },
      },
    },
    streaming: {
      on: {
        Begin: { actions: "rejectBegin" },
        Cancel: { target: "cancelling", actions: "requestCancel" },
        Finish: { target: "idle", actions: "closeTurn" },
      },
    },
    cancelling: {
      on: {
        Begin: { actions: "rejectBegin" },
        Finish: { target: "idle", actions: "closeTurn" },
      },
    },
  },
});

export const scenarioGroups = {
  "完整回合": [
    { name: "起始", events: [{ type: "Begin" }], expectStates: ["starting"], expectContext: { activeTurnId: 1 } },
    { name: "流式", events: [{ type: "StreamStarted" }], expectStates: ["streaming"] },
    { name: "收尾", events: [{ type: "Finish", reason: "done" }], expectStates: ["idle"], expectContext: { activeTurnId: null, finishedTurns: 1, lastFinishReason: "done" } },
  ],
  "忙时拒绝第二提交": [
    { name: "起始", events: [{ type: "Begin" }], expectStates: ["starting"] },
    { name: "第二提交被拒", events: [{ type: "Begin" }], expectStates: ["starting"], expectContext: { rejectedBegins: 1, activeTurnId: 1 } },
  ],
  "取消后收尾": [
    { name: "起始并流式", events: [{ type: "Begin" }, { type: "StreamStarted" }], expectStates: ["streaming"] },
    { name: "取消", events: [{ type: "Cancel" }], expectStates: ["cancelling"], expectContext: { cancelRequested: true } },
    { name: "中止收尾", events: [{ type: "Finish", reason: "aborted" }], expectStates: ["idle"], expectContext: { lastFinishReason: "aborted" } },
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
