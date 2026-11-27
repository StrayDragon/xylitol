/**
 * Print 模式输出状态机（Print Rendering）。
 *
 * 业务：非交互模式把 TextDelta 流式写 stdout、reasoning 写 stderr、工具摘要展示；
 * 消息去重、不回显用户输入、错误以非零退出码收尾。
 *
 * @spec llmanspec/specs/cli-print/cli-print.feature#@req:r34（streaming-output）
 * @spec llmanspec/specs/cli-print/cli-print.feature#@req:r43（tool-display）
 * @spec llmanspec/specs/cli-print/cli-print.feature#@req:r49（thinking-to-stderr）
 * @spec llmanspec/specs/cli-print/cli-print.feature#@req:r52（message-dedup）
 * @spec llmanspec/specs/cli-print/cli-print.feature#@req:r55（error-nonzero-exit）
 * @spec llmanspec/specs/cli-print/cli-print.feature#@req:r3325（no-user-echo）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1382（print-requires-prompt）
 *
 * 实现落点：crates/xylitol-app-print/。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, runScenarios, type ScenarioStep } from "../shared/conventions.ts";

export type PrintPhase = "awaitingPrompt" | "streaming" | "toolSummary" | "finished" | "failed";

export interface PrintContext {
  /** 用户提示是否存在。 */
  hasPrompt: boolean;
  /** stdout 累积文本。 */
  stdout: string;
  /** stderr 累积 reasoning。 */
  stderr: string;
  /** 已展示的工具摘要数。 */
  toolSummaries: number;
  /** 去重丢弃的重复消息数。 */
  dedupDropped: number;
  /** 回显用户输入的次数（须为 0）。 */
  userEchoes: number;
  /** 退出码。 */
  exitCode: number;
  /** 最近见过的消息 id。 */
  seenMessageIds: string[];
}

export const initialContext: PrintContext = {
  hasPrompt: true,
  stdout: "",
  stderr: "",
  toolSummaries: 0,
  dedupDropped: 0,
  userEchoes: 0,
  exitCode: 0,
  seenMessageIds: [],
};

export const meta = defineMeta({
  name: "print-rendering",
  pkg: "app-surface",
  purpose: "非交互输出：stdout 流式、reasoning 走 stderr、去重、非零退出",
  anchors: [
    "llmanspec/specs/cli-print/cli-print.feature#@req:r34",
    "llmanspec/specs/cli-print/cli-print.feature#@req:r43",
    "llmanspec/specs/cli-print/cli-print.feature#@req:r49",
    "llmanspec/specs/cli-print/cli-print.feature#@req:r52",
    "llmanspec/specs/cli-print/cli-print.feature#@req:r55",
    "llmanspec/specs/cli-print/cli-print.feature#@req:r3325",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1382",
  ],
  impl: ["crates/xylitol-app-print/"],
});

export const setupMachine = setup({
  types: {
    context: {} as PrintContext,
    events: {} as
      | { type: "start" }
      | { type: "textDelta"; text: string }
      | { type: "thinkingDelta"; text: string }
      | { type: "toolEnd"; summary: string }
      | { type: "messageStart"; id: string }
      | { type: "error"; kind: string }
      | { type: "done" }
  },
  guards: {
    promptPresent: ({ context }) => context.hasPrompt === true,
    promptMissing: ({ context }) => context.hasPrompt === false,
    messageIsNew: ({ context, event }) => !context.seenMessageIds.includes((event as { id: string }).id),
    messageIsDuplicate: ({ context, event }) => context.seenMessageIds.includes((event as { id: string }).id),
  },
  actions: {
    appendStdout: assign({ stdout: ({ context, event }) => context.stdout + (event as { text: string }).text }),
    appendStderr: assign({ stderr: ({ context, event }) => context.stderr + (event as { text: string }).text }),
    countTool: assign({ toolSummaries: ({ context }) => context.toolSummaries + 1 }),
    rememberMessage: assign({ seenMessageIds: ({ context, event }) => [...context.seenMessageIds, (event as { id: string }).id] }),
    dropDuplicate: assign({ dedupDropped: ({ context }) => context.dedupDropped + 1 }),
    failWith: assign({ exitCode: () => 1 }),
    finishOk: assign({ exitCode: () => 0 }),
  },
});

/** 按提示词有无生成机器：hasPrompt=false 走失败分支。 */
export function makePrintMachine(hasPrompt: boolean) {
  return setupMachine.createMachine({
    id: "print-rendering",
    initial: "awaitingPrompt",
    context: { ...initialContext, hasPrompt },
    states: printStates,
  });
}

const printStates = {
    awaitingPrompt: {
      on: {
        start: [
          { guard: "promptPresent", target: "streaming" },
          { guard: "promptMissing", target: "failed", actions: "failWith" },
        ],
      },
    },
    streaming: {
      on: {
        textDelta: { actions: "appendStdout" },
        thinkingDelta: { actions: "appendStderr" },
        toolEnd: { target: "toolSummary", actions: "countTool" },
        messageStart: [
          { guard: "messageIsNew", actions: "rememberMessage" },
          { guard: "messageIsDuplicate", actions: "dropDuplicate" },
        ],
        error: { target: "failed", actions: "failWith" },
        done: { target: "finished", actions: "finishOk" },
      },
    },
    toolSummary: {
      always: [{ target: "streaming" }],
    },
    finished: { type: "final" },
    failed: { type: "final" },
} as const;

/** 有提示词机器。 */
export const machine = makePrintMachine(true);

/** 缺提示词机器。 */
export const noPromptMachine = makePrintMachine(false);

export const scenarioGroups = {
  "流式输出到stdout": [
    { name: "启动", events: [{ type: "start" }], expectStates: ["streaming"] },
    { name: "两段文本增量", events: [{ type: "textDelta", text: "你好" }, { type: "textDelta", text: "，世界" }], expectStates: ["streaming"], expectContext: { stdout: "你好，世界" } },
    { name: "完成", events: [{ type: "done" }], expectStates: ["finished"], expectContext: { exitCode: 0 } },
  ],
  "reasoning写stderr": [
    { name: "启动并流式", events: [{ type: "start" }, { type: "thinkingDelta", text: "推理中" }], expectStates: ["streaming"], expectContext: { stderr: "推理中", stdout: "" } },
  ],
  "工具摘要展示": [
    { name: "启动并执行工具", events: [{ type: "start" }, { type: "toolEnd", summary: "read ok" }], expectStates: ["streaming"], expectContext: { toolSummaries: 1 } },
  ],
  "重复消息去重": [
    { name: "启动并收到同一消息两次", events: [{ type: "start" }, { type: "messageStart", id: "m1" }, { type: "messageStart", id: "m1" }], expectStates: ["streaming"], expectContext: { dedupDropped: 1 } },
  ],
  "错误非零退出": [
    { name: "启动并出错", events: [{ type: "start" }, { type: "error", kind: "provider" }], expectStates: ["failed"], expectContext: { exitCode: 1 } },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

if (import.meta.main) {
  const noPromptResult = runScenarios(noPromptMachine, [
    { name: "缺提示词启动失败", events: [{ type: "start" }], expectStates: ["failed"], expectContext: { exitCode: 1 } },
  ]);
  const results = { ...runScenarioGroups(machine, scenarioGroups), "无提示词启动失败": noPromptResult };
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
