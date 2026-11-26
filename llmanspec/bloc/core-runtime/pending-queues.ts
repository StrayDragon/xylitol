/**
 * 待处理队列状态机（Pending Queues：steer / follow-up）。
 *
 * 业务：会话信箱收编独立的 steer 与 follow_up 队列；ReAct 在模型调用前 drain steer，
 * turn 收尾时从 follow_up FIFO 弹一条作为下一次提交（新 turn，独立 AgentStart/AgentEnd）。
 * 队列项携带入队时分配的 message_id；仅未出队项可撤销。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1059（pending-queues）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3374（队列项逐项撤销）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1060（queue-update-events）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1028（abort 队列语义）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3322（one-submission-one-turn）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/capabilities/queue.rs（QueueMode / PendingMessageQueue / RevokeOutcome）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 队列模式：一次 drain 一条（缺省）或全部。 */
export type QueueMode = "oneAtATime" | "all";

export interface QueueItem {
  id: string;
  text: string;
}

export interface PendingQueuesContext {
  /** steer 队列（FIFO）。 */
  steer: QueueItem[];
  /** follow_up 队列（FIFO）。 */
  followUp: QueueItem[];
  /** drain 模式。 */
  mode: QueueMode;
  /** 下一个分配的队列项序号。 */
  nextSeq: number;
  /** 已被撤销的项 id 列表，用于断言撤销后不再进入任何 turn。 */
  revoked: string[];
  /** 已注入当前 turn 的 steer 条数。 */
  steerInjected: number;
  /** 已作为新 turn 弹出的 follow_up 条数。 */
  followUpConsumed: number;
}

export const initialContext: PendingQueuesContext = {
  steer: [],
  followUp: [],
  mode: "oneAtATime",
  nextSeq: 1,
  revoked: [],
  steerInjected: 0,
  followUpConsumed: 0,
};

export const meta = defineMeta({
  name: "pending-queues",
  pkg: "core-runtime",
  purpose: "steer 与 follow_up 双队列：入队、drain、逐项撤销、逐条成 turn",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1059",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3374",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1060",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1028",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3322",
  ],
  impl: ["crates/xylitol-runtime/src/agent/capabilities/queue.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as PendingQueuesContext,
    events: {} as
      | { type: "enqueueSteer"; text: string }
      | { type: "enqueueFollowUp"; text: string }
      | { type: "drainSteer" }
      | { type: "popFollowUp" }
      | { type: "revoke"; id: string }
      | { type: "clear" }
      | { type: "setMode"; mode: QueueMode }
  },
  guards: {
    steerNotEmpty: ({ context }) => context.steer.length > 0,
    followUpNotEmpty: ({ context }) => context.followUp.length > 0,
    isOneAtATime: ({ context }) => context.mode === "oneAtATime",
    steerHasMore: ({ context }) => context.steer.length > 0,
  },
  actions: {
    pushSteer: assign({
      steer: ({ context, event }) => [
        ...context.steer,
        { id: `q${context.nextSeq}`, text: (event as { text: string }).text },
      ],
      nextSeq: ({ context }) => context.nextSeq + 1,
    }),
    pushFollowUp: assign({
      followUp: ({ context, event }) => [
        ...context.followUp,
        { id: `q${context.nextSeq}`, text: (event as { text: string }).text },
      ],
      nextSeq: ({ context }) => context.nextSeq + 1,
    }),
    drainOneSteer: assign({
      steer: ({ context }) => context.steer.slice(1),
      steerInjected: ({ context }) => context.steerInjected + 1,
    }),
    drainAllSteer: assign({
      steer: () => [],
      steerInjected: ({ context }) => context.steerInjected + context.steer.length,
    }),
    popOneFollowUp: assign({
      followUp: ({ context }) => context.followUp.slice(1),
      followUpConsumed: ({ context }) => context.followUpConsumed + 1,
    }),
    recordRevoke: assign({
      revoked: ({ context, event }) => [...context.revoked, (event as { id: string }).id],
      steer: ({ context, event }) => context.steer.filter((i) => i.id !== (event as { id: string }).id),
      followUp: ({ context, event }) => context.followUp.filter((i) => i.id !== (event as { id: string }).id),
    }),
    clearAll: assign({
      steer: () => [],
      followUp: () => [],
    }),
    applyMode: assign({
      mode: ({ event }) => (event as { mode: QueueMode }).mode,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "pending-queues",
  initial: "accumulating",
  context: initialContext,
  states: {
    accumulating: {
      on: {
        enqueueSteer: { actions: "pushSteer" },
        enqueueFollowUp: { actions: "pushFollowUp" },
        setMode: { actions: "applyMode" },
        revoke: { actions: "recordRevoke" },
        clear: { actions: "clearAll" },
        drainSteer: [
          { guard: "steerNotEmpty", target: "steering" },
        ],
        popFollowUp: [
          { guard: "followUpNotEmpty", target: "newTurnPending" },
        ],
      },
    },
    steering: {
      on: {
        setMode: { actions: "applyMode" },
        revoke: { actions: "recordRevoke" },
      },
      always: [
        { guard: "isOneAtATime", target: "accumulating", actions: "drainOneSteer" },
        { guard: "steerHasMore", target: "accumulating", actions: "drainAllSteer" },
        { target: "accumulating" },
      ],
    },
    newTurnPending: {
      always: [{ target: "accumulating", actions: "popOneFollowUp" }],
    },
  },
});

export const scenarioGroups = {
  "改向在模型调用前注入": [
    { name: "入队 steer", events: [{ type: "enqueueSteer", text: "先看日志" }], expectStates: ["accumulating"], expectContext: { steerInjected: 0 } },
    { name: "drain 一条", events: [{ type: "drainSteer" }], expectStates: ["accumulating"], expectContext: { steerInjected: 1, steer: [] } },
  ],
  "缺省一次一条": [
    { name: "入队两条 steer", events: [{ type: "enqueueSteer", text: "a" }, { type: "enqueueSteer", text: "b" }] },
    { name: "缺省模式只 drain 一条", events: [{ type: "drainSteer" }], expectStates: ["accumulating"], expectContext: { steerInjected: 1 } },
    { name: "剩余一条仍在队列", expect: (s) => ((s as { context: PendingQueuesContext }).context.steer.length === 1 ? true : "期望剩 1 条") },
  ],
  "显式 all 模式一次清空": [
    { name: "入队两条并切 all", events: [{ type: "enqueueSteer", text: "a" }, { type: "enqueueSteer", text: "b" }, { type: "setMode", mode: "all" }], expectContext: { mode: "all" } },
    { name: "drain 全部", events: [{ type: "drainSteer" }], expectStates: ["accumulating"], expectContext: { steerInjected: 2 } },
  ],
  "追问逐条独立成回合": [
    { name: "入队两条 follow_up", events: [{ type: "enqueueFollowUp", text: "继续" }, { type: "enqueueFollowUp", text: "再继续" }], expectContext: { followUpConsumed: 0 } },
    { name: "弹一条成新 turn", events: [{ type: "popFollowUp" }], expectStates: ["accumulating"], expectContext: { followUpConsumed: 1 } },
    { name: "再弹一条", events: [{ type: "popFollowUp" }], expectStates: ["accumulating"], expectContext: { followUpConsumed: 2, followUp: [] } },
  ],
  "撤销未出队项": [
    { name: "入队两条", events: [{ type: "enqueueFollowUp", text: "第一条" }, { type: "enqueueFollowUp", text: "第二条" }] },
    { name: "撤销第一条", events: [{ type: "revoke", id: "q1" }], expectContext: { revoked: ["q1"] }, expect: (s) => {
        const c = (s as { context: PendingQueuesContext }).context;
        return c.followUp.length === 1 && c.followUp[0]?.id === "q2" ? true : "期望仅剩 q2";
      } },
    { name: "弹出的下一条是第二条", events: [{ type: "popFollowUp" }], expectStates: ["accumulating"], expectContext: { followUpConsumed: 1 } },
  ],
  "abort 清空 steer 保留 follow_up": [
    { name: "入队 steer 与 follow_up", events: [{ type: "enqueueSteer", text: "改向" }, { type: "enqueueFollowUp", text: "追问" }] },
    { name: "clear 只清 steer 语义由调用方决定", events: [{ type: "clear" }], expectContext: { steer: [], followUp: [] } },
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