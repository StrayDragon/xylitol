/**
 * 会话信箱状态机（Session Mailbox）。
 *
业务：每一会话槽由一个 actor 经有界命令信箱物理化单写者；提交按到达序逐条处理。
 * 信箱满时发送方等待（内部背压），不设对话队列容量上限。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3375（会话信箱单写者）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3322（one-submission-one-turn）
 * @spec llmanspec/specs/layer-architecture/layer-architecture.feature#@req:r1514（一 session 一写者）
 *
 * 实现落点：crates/xylitol-runtime/src/driver/in_process/actor.rs（MAILBOX_CAPACITY = 64）
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 信箱容量：与 Rust 侧 MAILBOX_CAPACITY 对齐。 */
export const MAILBOX_CAPACITY = 64;

export type MailboxPhase = "accepting" | "busy" | "draining" | "closed";

export interface MailboxContext {
  /** 已排队未消费的提交数。 */
  pending: number;
  /** 当前正在处理的提交数（0 或 1）。 */
  inFlight: number;
  /** 因信箱满而让发送方等待的次数。 */
  backpressureCount: number;
  /** 已消费提交总数。 */
  consumed: number;
  /** 是否已进入关闭。 */
  closed: boolean;
}

export const initialContext: MailboxContext = {
  pending: 0,
  inFlight: 0,
  backpressureCount: 0,
  consumed: 0,
  closed: false,
};

export const meta = defineMeta({
  name: "session-mailbox",
  pkg: "core-runtime",
  purpose: "有界命令信箱单写者：排队、背压、逐条消费",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3375",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3322",
    "llmanspec/specs/layer-architecture/layer-architecture.feature#@req:r1514",
  ],
  impl: ["crates/xylitol-runtime/src/driver/in_process/actor.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as MailboxContext,
    events: {} as
      | { type: "submit" }
      | { type: "pick" }
      | { type: "complete" }
      | { type: "close" }
  },
  guards: {
    canEnqueue: ({ context }) => context.pending < MAILBOX_CAPACITY,
    hasPending: ({ context }) => context.pending > 0,
  },
  actions: {
    enqueue: assign({ pending: ({ context }) => context.pending + 1 }),
    applyBackpressure: assign({ backpressureCount: ({ context }) => context.backpressureCount + 1 }),
    takeOne: assign({
      pending: ({ context }) => context.pending - 1,
      inFlight: () => 1,
    }),
    release: assign({
      inFlight: () => 0,
      consumed: ({ context }) => context.consumed + 1,
    }),
    markClosed: assign({ closed: () => true }),
  },
});

export const machine = setupMachine.createMachine({
  id: "session-mailbox",
  initial: "accepting",
  context: initialContext,
  states: {
    accepting: {
      on: {
        submit: [
          { guard: "canEnqueue", actions: "enqueue" },
          { actions: "applyBackpressure" },
        ],
        pick: [{ guard: "hasPending", target: "busy", actions: "takeOne" }],
        close: { target: "closed", actions: "markClosed" },
      },
    },
    busy: {
      on: {
        submit: [
          { guard: "canEnqueue", actions: "enqueue" },
          { actions: "applyBackpressure" },
        ],
        complete: { target: "draining", actions: "release" },
        close: { target: "closed", actions: "markClosed" },
      },
    },
    draining: {
      always: [{ guard: "hasPending", target: "busy", actions: "takeOne" }, { target: "accepting" }],
    },
    closed: {
      type: "final",
    },
  },
});

export const scenarioGroups = {
  "空闲连续提交排队": [
    { name: "第一条提交入队", events: [{ type: "submit" }], expectStates: ["accepting"], expectContext: { pending: 1, inFlight: 0 } },
    { name: "第二条提交排队", events: [{ type: "submit" }], expectStates: ["accepting"], expectContext: { pending: 2 } },
    { name: "取出第一条进入处理", events: [{ type: "pick" }], expectStates: ["busy"], expectContext: { pending: 1, inFlight: 1 } },
    { name: "第一条完成并取出第二条", events: [{ type: "complete" }], expectStates: ["busy"], expectContext: { pending: 0, inFlight: 1, consumed: 1 } },
    { name: "第二条完成回到接受", events: [{ type: "complete" }], expectStates: ["accepting"], expectContext: { consumed: 2, inFlight: 0 } },
  ],
  "信箱满背压": [
    { name: "灌满信箱", events: Array.from({ length: MAILBOX_CAPACITY }, () => ({ type: "submit" })), expectStates: ["accepting"], expectContext: { pending: MAILBOX_CAPACITY } },
    { name: "超出容量时发送方等待", events: [{ type: "submit" }], expectStates: ["accepting"], expectContext: { pending: MAILBOX_CAPACITY, backpressureCount: 1 } },
  ],
  "关闭后不再接受": [
    { name: "关闭", events: [{ type: "close" }], expectStates: ["closed"], expectContext: { closed: true } },
    { name: "关闭后提交无效", events: [{ type: "submit" }], expectStates: ["closed"], expectContext: { pending: 0 } },
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
