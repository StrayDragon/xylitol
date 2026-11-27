/**
 * 会话待办状态机（Agent Todo）。
 *
 * 业务：Todo 是会话内 SSOT，以 Custom latest-wins 持久；status 三态（pending / in_progress / completed）；
 * in_progress 可多条；content ≤ 80 字符；todo 工具受并发屏障约束；
 * 出站 AgentStatusBar 以 <todo> 子树只读投影当前清单。
 *
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1118（todo-domain-model）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1123（custom-agent-todo-latest-wins）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1126（multiple-in-progress-allowed）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1127（todo-tool-concurrency-barrier）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1128（status-bar-read-only-boundary）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1842（todo-content-max-length）
 * @spec llmanspec/specs/agent-todo/agent-todo.feature#@req:r1119（compact-preserves-todo-snapshot）
 *
 * 实现落点：plugins/xylitol-official-plugin-todo/、crates/xylitol-runtime/src/agent/prompt/status_bar.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type TodoStatus = "pending" | "inProgress" | "completed";

/** content 长度上限（r1842）。 */
export const TODO_CONTENT_MAX = 80;

export interface TodoItem {
  id: number;
  content: string;
  status: TodoStatus;
}

export interface TodoContext {
  /** 当前清单（latest-wins）。 */
  items: TodoItem[];
  /** 下一个分配 id。 */
  nextId: number;
  /** 被拒绝的写入次数（超长 / 非法状态）。 */
  rejectedWrites: number;
  /** 已持久化的 Custom 条目版本号。 */
  persistedVersion: number;
  /** 是否正在等待工具屏障。 */
  awaitingBarrier: boolean;
  /** 最近一次投影出的 <todo> 子树条数。 */
  projectedRows: number;
}

export const initialContext: TodoContext = {
  items: [],
  nextId: 1,
  rejectedWrites: 0,
  persistedVersion: 0,
  awaitingBarrier: false,
  projectedRows: 0,
};

export const meta = defineMeta({
  name: "agent-todo",
  pkg: "agent-domain",
  purpose: "待办清单 SSOT：latest-wins 持久、三态、多 in_progress、只读投影",
  anchors: [
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1118",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1123",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1126",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1127",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1128",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1842",
    "llmanspec/specs/agent-todo/agent-todo.feature#@req:r1119",
  ],
  impl: ["plugins/xylitol-official-plugin-todo/", "crates/xylitol-runtime/src/agent/prompt/status_bar.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as TodoContext,
    events: {} as
      | { type: "rewrite"; contents: string[] }
      | { type: "update"; id: number; status: TodoStatus }
      | { type: "enterBarrier" }
      | { type: "leaveBarrier" }
      | { type: "project" }
      | { type: "compact" }
  },
  guards: {
    contentsWithinLimit: ({ event }) => (event as { contents: string[] }).contents.every((c) => c.length <= TODO_CONTENT_MAX),
    contentsOverLimit: ({ event }) => (event as { contents: string[] }).contents.some((c) => c.length > TODO_CONTENT_MAX),
    inBarrier: ({ context }) => context.awaitingBarrier === true,
    notInBarrier: ({ context }) => context.awaitingBarrier === false,
  },
  actions: {
    rewriteAll: assign({
      items: ({ context, event }) => {
        const contents = (event as { contents: string[] }).contents;
        return contents.map((content, i) => ({ id: context.nextId + i, content, status: "pending" as TodoStatus }));
      },
      nextId: ({ context, event }) => context.nextId + (event as { contents: string[] }).contents.length,
      persistedVersion: ({ context }) => context.persistedVersion + 1,
    }),
    rejectWrite: assign({ rejectedWrites: ({ context }) => context.rejectedWrites + 1 }),
    applyStatus: assign({
      items: ({ context, event }) => {
        const e = event as { id: number; status: TodoStatus };
        return context.items.map((item) => (item.id === e.id ? { ...item, status: e.status } : item));
      },
      persistedVersion: ({ context }) => context.persistedVersion + 1,
    }),
    enterBarrier: assign({ awaitingBarrier: () => true }),
    leaveBarrier: assign({ awaitingBarrier: () => false }),
    projectRows: assign({ projectedRows: ({ context }) => context.items.length }),
    keepSnapshot: assign({ persistedVersion: ({ context }) => context.persistedVersion + 1 }),
  },
});

export const machine = setupMachine.createMachine({
  id: "agent-todo",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        rewrite: [
          { guard: "contentsWithinLimit", actions: "rewriteAll" },
          { guard: "contentsOverLimit", actions: "rejectWrite" },
        ],
        update: { actions: "applyStatus" },
        enterBarrier: { target: "barrierHeld", actions: "enterBarrier" },
        project: { actions: "projectRows" },
        compact: { actions: "keepSnapshot" },
      },
    },
    barrierHeld: {
      on: {
        leaveBarrier: { target: "idle", actions: "leaveBarrier" },
        rewrite: { actions: "rejectWrite" },
        update: { actions: "rejectWrite" },
      },
    },
  },
});

export const scenarioGroups = {
  "重写清单latest-wins": [
    { name: "首次重写", events: [{ type: "rewrite", contents: ["读代码", "改配置"] }], expectStates: ["idle"], expectContext: { persistedVersion: 1, projectedRows: 0 } },
    { name: "再次重写覆盖", events: [{ type: "rewrite", contents: ["只留一条"] }], expectStates: ["idle"], expectContext: { persistedVersion: 2 } },
    { name: "清单只剩一条", expect: (s) => ((s as { context: TodoContext }).context.items.length === 1 ? true : "期望仅 1 条") },
  ],
  "多条in_progress": [
    { name: "建立两条待办", events: [{ type: "rewrite", contents: ["甲", "乙"] }] },
    { name: "两条同时置为进行中", events: [{ type: "update", id: 1, status: "inProgress" }, { type: "update", id: 2, status: "inProgress" }], expectStates: ["idle"], expect: (s) => {
        const c = (s as { context: TodoContext }).context;
        return c.items.filter((i) => i.status === "inProgress").length === 2 ? true : "期望两条 in_progress";
      } },
  ],
  "超长内容被拒绝": [
    { name: "提交超 80 字符内容", events: [{ type: "rewrite", contents: ["x".repeat(81)] }], expectStates: ["idle"], expectContext: { rejectedWrites: 1 } },
    { name: "清单未变", expect: (s) => ((s as { context: TodoContext }).context.items.length === 0 ? true : "期望清单为空") },
  ],
  "并发屏障期拒写": [
    { name: "进入屏障", events: [{ type: "enterBarrier" }], expectStates: ["barrierHeld"], expectContext: { awaitingBarrier: true } },
    { name: "屏障内写入被拒", events: [{ type: "update", id: 1, status: "completed" }], expectStates: ["barrierHeld"], expectContext: { rejectedWrites: 1 } },
    { name: "离开屏障", events: [{ type: "leaveBarrier" }], expectStates: ["idle"], expectContext: { awaitingBarrier: false } },
  ],
  "只读投影": [
    { name: "建立三条待办", events: [{ type: "rewrite", contents: ["甲", "乙", "丙"] }] },
    { name: "投影出三行", events: [{ type: "project" }], expectStates: ["idle"], expectContext: { projectedRows: 3 } },
    { name: "投影不改持久版本", expect: (s) => ((s as { context: TodoContext }).context.persistedVersion === 1 ? true : "期望持久版本仍为 1") },
  ],
  "压缩保留快照": [
    { name: "建立待办", events: [{ type: "rewrite", contents: ["甲"] }] },
    { name: "压缩后快照保留", events: [{ type: "compact" }], expectStates: ["idle"], expect: (s) => {
        const c = (s as { context: TodoContext }).context;
        return c.items.length === 1 && c.persistedVersion === 2 ? true : "期望清单与版本保留";
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
