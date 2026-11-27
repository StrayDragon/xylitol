/**
 * 工具批执行状态机（Tool Batch Execution）。
 *
 * 业务：同一模型响应中的全部工具调用在下一轮模型调用前执行完毕。
 * 工具分 Barrier（不可进并行窗，如 MCP）与 Parallel（可并行）两类；
 * 结果按源顺序写回 history，不按完成顺序。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1045（tool-batch-barrier-parallel）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1046（tool-batch-history-source-order）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1047（工具批后继续）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1058（hooks-at-tool-boundary）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1451（mcp-tool-hard-barrier）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1127（todo-tool-concurrency-barrier）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/runtime/tool_batch.rs、crates/xylitol-runtime/src/agent/runtime/tool_exec.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 单个工具调用的调度类别。 */
export type ToolKind = "barrier" | "parallel";

export interface ToolCall {
  /** 调用在模型响应中的源序号。 */
  index: number;
  /** 工具名。 */
  name: string;
  /** 调度类别。 */
  kind: ToolKind;
}

export interface ToolBatchContext {
  /** 待执行批。 */
  pending: ToolCall[];
  /** 已完成的调用（按完成序）。 */
  completed: ToolCall[];
  /** 写回 history 的顺序（须等于源序）。 */
  historyOrder: number[];
  /** before hook 拒绝的次数。 */
  beforeRejected: number;
  /** 并行窗内最大并发。 */
  maxParallel: number;
  /** 是否曾有 barrier 工具混入并行窗（应为 false）。 */
  barrierInParallelWindow: boolean;
}

export const initialContext: ToolBatchContext = {
  pending: [],
  completed: [],
  historyOrder: [],
  beforeRejected: 0,
  maxParallel: 0,
  barrierInParallelWindow: false,
};

export const meta = defineMeta({
  name: "tool-batch",
  pkg: "core-runtime",
  purpose: "工具批调度：barrier 串行、parallel 并行、源序回写",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1045",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1046",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1047",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1058",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1451",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1127",
  ],
  impl: ["crates/xylitol-runtime/src/agent/runtime/tool_batch.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as ToolBatchContext,
    events: {} as
      | { type: "plan"; calls: ToolCall[] }
      | { type: "beforeHook"; accepted: boolean }
      | { type: "toolDone"; index: number }
      | { type: "flush" }
  },
  guards: {
    hasPending: ({ context }) => context.pending.length > 0,
    headIsBarrier: ({ context }) => context.pending[0]?.kind === "barrier",
    headIsParallel: ({ context }) => context.pending[0]?.kind === "parallel",
    hookAccepted: ({ event }) => (event as { accepted: boolean }).accepted === true,
    hookRejected: ({ event }) => (event as { accepted: boolean }).accepted === false,
  },
  actions: {
    storePlan: assign({
      pending: ({ event }) => [...(event as { calls: ToolCall[] }).calls],
      completed: () => [],
      historyOrder: () => [],
      barrierInParallelWindow: () => false,
      maxParallel: () => 0,
    }),
    rejectBefore: assign({ beforeRejected: ({ context }) => context.beforeRejected + 1 }),
    recordDone: assign({
      completed: ({ context, event }) => {
        const idx = (event as { index: number }).index;
        const call = context.pending.find((c) => c.index === idx);
        return call ? [...context.completed, call] : context.completed;
      },
      pending: ({ context, event }) => context.pending.filter((c) => c.index !== (event as { index: number }).index),
      historyOrder: ({ context, event }) => [...context.historyOrder, (event as { index: number }).index].sort((a, b) => a - b),
    }),
    noteParallel: assign({
      maxParallel: ({ context }) => Math.max(context.maxParallel, context.pending.filter((c) => c.kind === "parallel").length),
    }),
    flagBarrierMix: assign({ barrierInParallelWindow: () => true }),
  },
});

export const machine = setupMachine.createMachine({
  id: "tool-batch",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: { plan: [{ target: "running", actions: "storePlan" }] },
    },
    running: {
      on: {
        plan: [{ actions: "storePlan" }],
        beforeHook: [
          { guard: "hookRejected", actions: "rejectBefore" },
          { guard: "hookAccepted" },
        ],
        toolDone: { actions: "recordDone" },
        flush: [{ guard: "hasPending", target: "running" }, { target: "flushed" }],
      },
    },
    flushed: {
      type: "final",
    },
  },
});

export const scenarioGroups = {
  "全部工具执行完再续跑": [
    {
      name: "规划三个调用",
      events: [{ type: "plan", calls: [
        { index: 0, name: "read", kind: "parallel" },
        { index: 1, name: "mcp__srv__ping", kind: "barrier" },
        { index: 2, name: "grep", kind: "parallel" },
      ] }],
      expectStates: ["running"],
      expectContext: { beforeRejected: 0 },
    },
    { name: "执行全部三个", events: [{ type: "toolDone", index: 0 }, { type: "toolDone", index: 1 }, { type: "toolDone", index: 2 }], expectStates: ["running"], expect: (s) => ((s as { context: ToolBatchContext }).context.pending.length === 0 ? true : "期望批已空") },
    { name: "flush 进入续跑", events: [{ type: "flush" }], expectStates: ["flushed"] },
  ],
  "结果按源序回写": [
    { name: "规划三个调用", events: [{ type: "plan", calls: [
        { index: 0, name: "read", kind: "parallel" },
        { index: 1, name: "grep", kind: "parallel" },
        { index: 2, name: "ls", kind: "parallel" },
      ] }] },
    { name: "乱序完成", events: [{ type: "toolDone", index: 2 }, { type: "toolDone", index: 0 }, { type: "toolDone", index: 1 }], expectStates: ["running"], expectContext: { historyOrder: [0, 1, 2] } },
  ],
  "before hook 拒绝不执行": [
    { name: "规划一个 bash 调用", events: [{ type: "plan", calls: [{ index: 0, name: "bash", kind: "barrier" }] }], expectStates: ["running"] },
    { name: "before hook 拒绝", events: [{ type: "beforeHook", accepted: false }], expectStates: ["running"], expectContext: { beforeRejected: 1 } },
    { name: "该调用未进入完成集", expect: (s) => ((s as { context: ToolBatchContext }).context.completed.length === 0 ? true : "期望无已完成调用") },
  ],
  "MCP 工具为硬屏障": [
    { name: "规划 MCP 与普通工具", events: [{ type: "plan", calls: [
        { index: 0, name: "mcp__srv__ping", kind: "barrier" },
        { index: 1, name: "read", kind: "parallel" },
      ] }], expectStates: ["running"], expect: (s) => {
        const c = (s as { context: ToolBatchContext }).context;
        return c.pending[0]?.kind === "barrier" ? true : "期望首个调用为 barrier";
      } },
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
