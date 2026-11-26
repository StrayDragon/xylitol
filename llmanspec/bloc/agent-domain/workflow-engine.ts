/**
 * Workflow 执行状态机（Workflow Engine）。
 *
 * 业务：带依赖的任务图、局部重规划与审批。容器阶段必须收敛为 WorkflowPhase：
 * Contract / Executing / AwaitingApproval / Replanning / SubWorkflow / Summarizing / Done。
 * 任务状态写入必须经转移表：Pending→InProgress；InProgress→Completed/Failed/Blocked；
 * Blocked→InProgress（批准消费）；Failed→InProgress（仅 replan 入口）；Completed 锁定。
 * 任务批准必须一次消费一次（解锁即清位），重跑必须重新批准。
 *
 * @spec llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2946（workflow-middleware）
 * @spec llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2966（workflow-tools）
 * @spec llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2969（nested-workflow）
 *
 * 实现落点：plugins/xylitol-official-plugin-workflow/。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 容器阶段闭集（spec 明确要求收敛到这七个）。 */
export type WorkflowPhase =
  | "contract"
  | "executing"
  | "awaitingApproval"
  | "replanning"
  | "subWorkflow"
  | "summarizing"
  | "done";

export type TaskStatus = "pending" | "inProgress" | "completed" | "failed" | "blocked";

export interface WorkflowTask {
  id: string;
  status: TaskStatus;
}

export interface WorkflowContext {
  /** 当前容器阶段。 */
  phase: WorkflowPhase;
  /** 任务列表。 */
  tasks: WorkflowTask[];
  /** 已用重规划次数。 */
  replansUsed: number;
  /** 重规划上限。 */
  maxReplans: number;
  /** 待消费的批准（一次消费一次）。 */
  approvalPending: boolean;
  /** 是否已总结过。 */
  summarized: boolean;
  /** 被拒绝的非法写入次数。 */
  rejectedWrites: number;
  /** 停止原因。 */
  stopReason: string | null;
  /** 嵌套深度。 */
  depth: number;
  /** 嵌套总步数上限。 */
  maxTotalSteps: number;
}

export const initialContext: WorkflowContext = {
  phase: "contract",
  tasks: [],
  replansUsed: 0,
  maxReplans: 3,
  approvalPending: false,
  summarized: false,
  rejectedWrites: 0,
  stopReason: null,
  depth: 0,
  maxTotalSteps: 50,
};

export const meta = defineMeta({
  name: "workflow-engine",
  pkg: "agent-domain",
  purpose: "任务图执行：七阶段容器、任务转移表、批准一次消费",
  anchors: [
    "llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2946",
    "llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2966",
    "llmanspec/specs/agent-workflow/agent-workflow.feature#@req:r2969",
  ],
  impl: ["plugins/xylitol-official-plugin-workflow/"],
});

export const setupMachine = setup({
  types: {
    context: {} as WorkflowContext,
    events: {} as
      | { type: "writePlan"; tasks: { id: string; status: TaskStatus }[] }
      | { type: "taskCompleted"; id: string }
      | { type: "taskFailed"; id: string }
      | { type: "taskBlocked"; id: string }
      | { type: "userApproved" }
      | { type: "replan" }
      | { type: "enterSubWorkflow" }
      | { type: "exitSubWorkflow" }
      | { type: "summarize" }
  },
  guards: {
    planIsDag: ({ event }) => {
      const tasks = (event as { tasks: { id: string }[] }).tasks;
      const ids = new Set(tasks.map((t) => t.id));
      return ids.size === tasks.length;
    },
    planHasDuplicateIds: ({ event }) => {
      const tasks = (event as { tasks: { id: string }[] }).tasks;
      const ids = new Set(tasks.map((t) => t.id));
      return ids.size !== tasks.length;
    },
    approvalAvailable: ({ context }) => context.approvalPending === true,
    replansRemain: ({ context }) => context.replansUsed < context.maxReplans,
    replansExhausted: ({ context }) => context.replansUsed >= context.maxReplans,
    allCompleted: ({ context }) => context.tasks.length > 0 && context.tasks.every((t) => t.status === "completed"),
    notSummarized: ({ context }) => context.summarized === false,
    targetNotCompleted: ({ context, event }) => {
      const id = (event as { id: string }).id;
      return context.tasks.find((t) => t.id === id)?.status !== "completed";
    },
    targetCompleted: ({ context, event }) => {
      const id = (event as { id: string }).id;
      return context.tasks.find((t) => t.id === id)?.status === "completed";
    },
  },
  actions: {
    adoptPlan: assign({
      tasks: ({ event }) => (event as { tasks: { id: string; status: TaskStatus }[] }).tasks.map((t) => ({ ...t })),
      phase: () => "executing" as WorkflowPhase,
    }),
    rejectPlan: assign({ rejectedWrites: ({ context }) => context.rejectedWrites + 1 }),
    setTaskStatus: assign({
      tasks: ({ context, event }) => {
        const e = event as { id: string };
        const status: TaskStatus =
          event.type === "taskCompleted" ? "completed" : event.type === "taskFailed" ? "failed" : "blocked";
        return context.tasks.map((t) => (t.id === e.id ? { ...t, status } : t));
      },
    }),
    rejectCompletedWrite: assign({ rejectedWrites: ({ context }) => context.rejectedWrites + 1 }),
    markAwaiting: assign({ phase: () => "awaitingApproval" as WorkflowPhase }),
    consumeApproval: assign({
      approvalPending: () => false,
      phase: () => "executing" as WorkflowPhase,
      tasks: ({ context }) => context.tasks.map((t) => (t.status === "blocked" ? { ...t, status: "inProgress" as TaskStatus } : t)),
    }),
    enterReplan: assign({
      phase: () => "replanning" as WorkflowPhase,
      replansUsed: ({ context }) => context.replansUsed + 1,
    }),
    enterSub: assign({ phase: () => "subWorkflow" as WorkflowPhase, depth: ({ context }) => context.depth + 1 }),
    leaveSub: assign({ phase: () => "executing" as WorkflowPhase, depth: ({ context }) => Math.max(0, context.depth - 1) }),
    enterSummary: assign({ phase: () => "summarizing" as WorkflowPhase }),
    finishDone: assign({ phase: () => "done" as WorkflowPhase, summarized: () => true }),
    stopWith: assign({ stopReason: ({ event }) => event.type }),
    clearApprovalOnBlock: assign({ approvalPending: () => true }),
  },
});

export const machine = setupMachine.createMachine({
  id: "workflow-engine",
  initial: "contract",
  context: initialContext,
  states: {
    contract: {
      on: {
        writePlan: [
          { guard: "planIsDag", target: "executing", actions: "adoptPlan" },
          { guard: "planHasDuplicateIds", actions: "rejectPlan" },
        ],
      },
    },
    executing: {
      on: {
        taskCompleted: [
          { guard: "targetNotCompleted", actions: "setTaskStatus" },
          { guard: "targetCompleted", actions: "rejectCompletedWrite" },
        ],
        taskFailed: { actions: "setTaskStatus" },
        taskBlocked: { target: "awaitingApproval", actions: ["setTaskStatus", "clearApprovalOnBlock"] },
        replan: [
          { guard: "replansRemain", target: "replanning", actions: "enterReplan" },
          { guard: "replansExhausted", target: "done", actions: ["stopWith"] },
        ],
        enterSubWorkflow: { target: "subWorkflow", actions: "enterSub" },
        summarize: { target: "summarizing", actions: "enterSummary" },
        writePlan: { actions: "rejectPlan" },
      },
    },
    awaitingApproval: {
      on: {
        userApproved: { target: "executing", actions: "consumeApproval" },
        writePlan: { actions: "rejectPlan" },
      },
    },
    replanning: {
      on: {
        writePlan: [
          { guard: "planIsDag", target: "executing", actions: "adoptPlan" },
          { guard: "planHasDuplicateIds", actions: "rejectPlan" },
        ],
      },
    },
    subWorkflow: {
      on: {
        exitSubWorkflow: { target: "executing", actions: "leaveSub" },
        writePlan: { actions: "rejectPlan" },
      },
    },
    summarizing: {
      on: {
        summarize: { target: "done", actions: "finishDone" },
      },
    },
    done: {
      type: "final",
    },
  },
});

export const scenarioGroups = {
  "写入计划后进入任务指令": [
    { name: "写入含 pending 任务的计划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"], expectContext: { phase: "executing" } },
  ],
  "重复 id 计划被拒": [
    { name: "写入重复 id 计划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }, { id: "t1", status: "pending" }] }], expectStates: ["contract"], expectContext: { rejectedWrites: 1 } },
  ],
  "任务失败后局部重规划": [
    { name: "写入计划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"] },
    { name: "标记任务失败", events: [{ type: "taskFailed", id: "t1" }], expectStates: ["executing"] },
    { name: "进入重规划", events: [{ type: "replan" }], expectStates: ["replanning"], expectContext: { replansUsed: 1 } },
    { name: "重规划计划被采用", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"] },
  ],
  "阻塞任务等待批准": [
    { name: "写入计划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"] },
    { name: "标记阻塞", events: [{ type: "taskBlocked", id: "t1" }], expectStates: ["awaitingApproval"], expectContext: { approvalPending: true } },
    { name: "批准一次解锁", events: [{ type: "userApproved" }], expectStates: ["executing"], expectContext: { approvalPending: false } },
    { name: "批准已消费不重复解锁", expect: (s) => {
        const c = (s as { context: WorkflowContext }).context;
        return c.approvalPending === false && c.tasks[0]?.status === "inProgress" ? true : "期望已解锁且批准清位";
      } },
  ],
  "Completed锁定拒写": [
    { name: "写入计划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"] },
    { name: "标记完成", events: [{ type: "taskCompleted", id: "t1" }], expectStates: ["executing"] },
    { name: "再次写 completed 被拒", events: [{ type: "taskCompleted", id: "t1" }], expectStates: ["executing"], expectContext: { rejectedWrites: 1 } },
  ],
  "嵌套子workflow": [
    { name: "写入计划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"] },
    { name: "进入子 workflow", events: [{ type: "enterSubWorkflow" }], expectStates: ["subWorkflow"], expectContext: { depth: 1 } },
    { name: "子 workflow 结束", events: [{ type: "exitSubWorkflow" }], expectStates: ["executing"], expectContext: { depth: 0 } },
  ],
  "全部完成后总结": [
    { name: "写入计划并完成", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }, { type: "taskCompleted", id: "t1" }], expectStates: ["executing"] },
    { name: "进入总结", events: [{ type: "summarize" }], expectStates: ["summarizing"] },
    { name: "总结完成收尾", events: [{ type: "summarize" }], expectStates: ["done"], expectContext: { summarized: true } },
  ],
  "重规划用尽停止": [
    { name: "写入计划并耗尽重规划", events: [{ type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }, { type: "replan" }, { type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }, { type: "replan" }, { type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }, { type: "replan" }, { type: "writePlan", tasks: [{ id: "t1", status: "pending" }] }], expectStates: ["executing"], expectContext: { replansUsed: 3 } },
    { name: "第四次重规划被拒并停止", events: [{ type: "replan" }], expectStates: ["done"], expectContext: { stopReason: "replan" } },
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