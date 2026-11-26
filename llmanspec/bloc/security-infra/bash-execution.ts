/**
 * bash 子进程执行状态机（Bash Execution）。
 *
 * 业务：经 shell 子进程执行命令，流式回传输出块；有界通道满时发送方合并；
 * 取消杀整个进程组；超时逐级升级（SIGTERM → 5 秒宽限 → SIGKILL），未提供
 * 超时不得触发升级；输出截断到上限并把完整输出溢出到临时文件、追加脚注；
 * 结果以显式结局枚举表达，exit_code 只承载真实退出码。
 *
 * @spec llmanspec/specs/infra-bash/infra-bash.feature#@req:r1429（流式执行器与有界通道合并）
 * @spec llmanspec/specs/infra-bash/infra-bash.feature#@req:r1430（取消杀进程组并标 cancelled）
 * @spec llmanspec/specs/infra-bash/infra-bash.feature#@req:r1431（输出截断、溢出临时文件与脚注）
 * @spec llmanspec/specs/infra-bash/infra-bash.feature#@req:r1428（超时逐级升级，无超时不升级）
 * @spec llmanspec/specs/infra-bash/infra-bash.feature#@req:r1435（运行时可达 abort 触发杀树）
 * @spec llmanspec/specs/infra-bash/infra-bash.feature#@req:r3351（显式结局枚举与 exit_code 单义）
 * @spec llmanspec/specs/agent-tools/agent-tools.feature#@req:r1160（CancellationToken 杀树并合并流式输出）
 *
 * 实现落点：plugins/xylitol-official-plugin-tools/src/tools/bash.rs、
 * crates/xylitol-runtime/src/driver/bang_exec.rs。
 *
 * 建模说明：guard 在动作执行前求值——overLimit 读的是此前 stdout 事件累加
 * 后的 bytesProduced，本事件不改字节量，故 guard 与动作看到同一口径；
 * 「Truncated: N lines shown」按事件负载的行数口径记录（模型不携带真实
 * 文本内容）；脚注格式对齐 r1431 的 pi 形字符串。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 截断上限缺省值：与内置工具截断同口径（50 KiB），可由 spawn 覆盖。 */
export const DEFAULT_TRUNCATE_BYTES = 50 * 1024;

/** SIGTERM 后的宽限窗口长度；到期 SIGKILL（r1428）。 */
export const SIGTERM_GRACE_SECONDS = 5;

export type BashOutcome = "completed" | "terminated" | "timed-out";

export interface SpawnEvent {
  type: "spawn";
  maxBytes?: number;
  timeoutMs?: number;
}

export interface StdoutEvent {
  type: "stdout";
  bytes: number;
  lines?: number;
  /** 有界通道 Full 时发送方合并本次写入（r1429）。 */
  full?: boolean;
}

export interface FinishEvent {
  type: "finish";
  exitCode: number;
}

export type BashEvent =
  | SpawnEvent
  | StdoutEvent
  | { type: "outputLimitExceeded" }
  | { type: "cancel" }
  | { type: "timeoutSigterm" }
  | { type: "graceExpired" }
  | { type: "exitGracefully"; exitCode: number }
  | FinishEvent;

export interface BashContext {
  maxBytes: number;
  timeoutArmed: boolean;
  bytesProduced: number;
  linesProduced: number;
  chunkWrites: number;
  mergedWrites: number;
  outcome: BashOutcome | null;
  cancelled: boolean;
  exitCode: number | null;
  truncated: boolean;
  spillPath: string | null;
  footer: string | null;
  spills: number;
  /** 无超时时拒绝超时升级的次数（可观测，r1428/r1436）。 */
  escalationRefused: number;
  /** 误发 outputLimitExceeded 且未超限的次数。 */
  ignoredLimitSignals: number;
}

export const initialContext: BashContext = {
  maxBytes: DEFAULT_TRUNCATE_BYTES,
  timeoutArmed: false,
  bytesProduced: 0,
  linesProduced: 0,
  chunkWrites: 0,
  mergedWrites: 0,
  outcome: null,
  cancelled: false,
  exitCode: null,
  truncated: false,
  spillPath: null,
  footer: null,
  spills: 0,
  escalationRefused: 0,
  ignoredLimitSignals: 0,
};

export const meta = defineMeta({
  name: "bash-execution",
  pkg: "security-infra",
  purpose: "bash 子进程执行：流式输出、取消杀树、超时升级、截断溢出与显式结局",
  anchors: [
    "llmanspec/specs/infra-bash/infra-bash.feature#@req:r1429",
    "llmanspec/specs/infra-bash/infra-bash.feature#@req:r1430",
    "llmanspec/specs/infra-bash/infra-bash.feature#@req:r1431",
    "llmanspec/specs/infra-bash/infra-bash.feature#@req:r1428",
    "llmanspec/specs/infra-bash/infra-bash.feature#@req:r1435",
    "llmanspec/specs/infra-bash/infra-bash.feature#@req:r3351",
    "llmanspec/specs/agent-tools/agent-tools.feature#@req:r1160",
  ],
  impl: [
    "plugins/xylitol-official-plugin-tools/src/tools/bash.rs",
    "crates/xylitol-runtime/src/driver/bang_exec.rs",
  ],
});

/** r1431 脚注：path 不可用时用 (unavailable)。 */
export function truncationFooter(spillPath: string, linesShown: number, limitBytes: number): string {
  return `[Full output: ${spillPath}. Truncated: ${linesShown} lines shown (${limitBytes} limit)]`;
}

function spillFile(context: BashContext): { spillPath: string; spills: number } {
  const spillPath = `/tmp/xylitol-bash-spill-${context.spills + 1}.out`;
  return { spillPath, spills: context.spills + 1 };
}

export const setupMachine = setup({
  types: {
    context: {} as BashContext,
    events: {} as BashEvent,
  },
  guards: {
    // guard 先于动作求值：字节量由此前 stdout 事件累加，本事件不改它。
    overLimit: ({ context }) => context.bytesProduced > context.maxBytes,
    timeoutArmed: ({ context }) => context.timeoutArmed,
  },
  actions: {
    begin: assign(({ event }) => {
      const e = event as SpawnEvent;
      return {
        maxBytes: e.maxBytes ?? DEFAULT_TRUNCATE_BYTES,
        timeoutArmed: typeof e.timeoutMs === "number",
        bytesProduced: 0,
        linesProduced: 0,
        chunkWrites: 0,
        mergedWrites: 0,
        outcome: null,
        cancelled: false,
        exitCode: null,
        truncated: false,
        spillPath: null,
        footer: null,
      };
    }),
    streamChunk: assign(({ context, event }) => {
      const e = event as StdoutEvent;
      return {
        bytesProduced: context.bytesProduced + e.bytes,
        linesProduced: context.linesProduced + (e.lines ?? 0),
        chunkWrites: context.chunkWrites + 1,
        mergedWrites: context.mergedWrites + (e.full === true ? 1 : 0),
      };
    }),
    killOverLimit: assign(({ context }) => ({
      outcome: "terminated" as const,
      cancelled: false,
      exitCode: null,
      truncated: true,
      ...spillFile(context),
      footer: truncationFooter(
          `/tmp/xylitol-bash-spill-${context.spills + 1}.out`,
          context.linesProduced,
          context.maxBytes,
      ),
    })),
    ignoreLimitSignal: assign(({ context }) => ({
      ignoredLimitSignals: context.ignoredLimitSignals + 1,
    })),
    cancelKill: assign({
      outcome: () => "terminated" as const,
      cancelled: () => true,
      exitCode: () => null,
    }),
    refuseEscalation: assign(({ context }) => ({
      escalationRefused: context.escalationRefused + 1,
    })),
    finishTruncated: assign(({ context, event }) => ({
      outcome: "completed" as const,
      exitCode: (event as FinishEvent).exitCode,
      truncated: true,
      ...spillFile(context),
      footer: truncationFooter(
          `/tmp/xylitol-bash-spill-${context.spills + 1}.out`,
          context.linesProduced,
          context.maxBytes,
      ),
    })),
    finishPlain: assign(({ event }) => ({
      outcome: "completed" as const,
      exitCode: (event as FinishEvent).exitCode,
    })),
    finishGrace: assign(({ event }) => ({
      outcome: "completed" as const,
      exitCode: (event as { exitCode: number }).exitCode,
    })),
    sigkill: assign({
      outcome: () => "timed-out" as const,
      exitCode: () => null,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "bash-execution",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        spawn: { target: "running", actions: "begin" },
      },
    },
    running: {
      on: {
        stdout: { actions: "streamChunk" },
        outputLimitExceeded: [
          { guard: "overLimit", target: "killed", actions: "killOverLimit" },
          { actions: "ignoreLimitSignal" },
        ],
        // r1430/r1435/r1160：取消杀整个进程组，结果标 cancelled。
        cancel: { target: "killed", actions: "cancelKill" },
        timeoutSigterm: [
          // r1428：只有调用方提供超时才走升级路径。
          { guard: "timeoutArmed", target: "grace" },
          { actions: "refuseEscalation" },
        ],
        finish: [
          { guard: "overLimit", target: "finished", actions: "finishTruncated" },
          { target: "finished", actions: "finishPlain" },
        ],
      },
    },
    grace: {
      on: {
        exitGracefully: { target: "finished", actions: "finishGrace" },
        graceExpired: { target: "timedOut", actions: "sigkill" },
      },
    },
    finished: { type: "final" },
    killed: { type: "final" },
    timedOut: { type: "final" },
  },
});

export const scenarioGroups = {
  "流式输出回传完成": [
    {
      name: "spawn 后两次流式输出（r1429）",
      events: [
        { type: "spawn", maxBytes: 1024 },
        { type: "stdout", bytes: 4, lines: 1 },
        { type: "stdout", bytes: 5, lines: 1 },
      ],
      expectStates: ["running"],
      expectContext: { bytesProduced: 9, chunkWrites: 2, mergedWrites: 0 },
    },
    {
      name: "正常退出，未截断无脚注",
      events: [{ type: "finish", exitCode: 0 }],
      expectStates: ["finished"],
      expectContext: { outcome: "completed", exitCode: 0, truncated: false, footer: null },
    },
  ],
  "有界通道满合并": [
    {
      name: "通道满时发送方合并写入并计数（r1429/r1160）",
      events: [
        { type: "spawn", maxBytes: 1024 },
        { type: "stdout", bytes: 10, lines: 1, full: true },
        { type: "stdout", bytes: 10, lines: 1, full: true },
        { type: "finish", exitCode: 0 },
      ],
      expectStates: ["finished"],
      expectContext: { mergedWrites: 2, chunkWrites: 2, outcome: "completed" },
    },
  ],
  "超限截断溢出临时文件": [
    {
      name: "输出累计超过上限（r1431）",
      events: [
        { type: "spawn", maxBytes: 100 },
        { type: "stdout", bytes: 80, lines: 2 },
        { type: "stdout", bytes: 80, lines: 2 },
      ],
      expectStates: ["running"],
      expectContext: { bytesProduced: 160 },
    },
    {
      name: "退出时截断尾部、溢出到临时文件并追加脚注",
      events: [{ type: "finish", exitCode: 0 }],
      expectStates: ["finished"],
      expectContext: { truncated: true, spillPath: "/tmp/xylitol-bash-spill-1.out", outcome: "completed", exitCode: 0 },
      expect: (snapshot) => {
        const ctx = (snapshot as { context: BashContext }).context;
        const want = "[Full output: /tmp/xylitol-bash-spill-1.out. Truncated: 4 lines shown (100 limit)]";
        return ctx.footer === want ? true : `脚注不合口径：${String(ctx.footer)}`;
      },
    },
  ],
  "stdout 超限杀进程": [
    {
      name: "超限信号在超限前提下杀进程并返回截断输出（r1428）",
      events: [
        { type: "spawn", maxBytes: 100 },
        { type: "stdout", bytes: 200, lines: 5 },
        { type: "outputLimitExceeded" },
      ],
      expectStates: ["killed"],
      expectContext: { outcome: "terminated", truncated: true, exitCode: null },
    },
  ],
  "取消杀树": [
    {
      name: "取消后杀进程组并标 cancelled（r1430/r1435/r1160）",
      events: [
        { type: "spawn", maxBytes: 1024 },
        { type: "stdout", bytes: 10, lines: 1 },
        { type: "cancel" },
      ],
      expectStates: ["killed"],
      expectContext: { cancelled: true, outcome: "terminated", exitCode: null },
    },
  ],
  "超时升级 SIGTERM 到 SIGKILL": [
    {
      name: "提供超时时 SIGTERM 进入宽限窗（r1428）",
      events: [
        { type: "spawn", maxBytes: 1024, timeoutMs: 500 },
        { type: "timeoutSigterm" },
      ],
      expectStates: ["grace"],
      expectContext: { timeoutArmed: true },
    },
    {
      name: "宽限到期 SIGKILL，结局为超时且无退出码（r3351）",
      events: [{ type: "graceExpired" }],
      expectStates: ["timedOut"],
      expectContext: { outcome: "timed-out", exitCode: null },
    },
  ],
  "宽限期内自行退出": [
    {
      name: "SIGTERM 宽限窗内进程自行退出，记真实退出码",
      events: [
        { type: "spawn", maxBytes: 1024, timeoutMs: 500 },
        { type: "timeoutSigterm" },
        { type: "exitGracefully", exitCode: 0 },
      ],
      expectStates: ["finished"],
      expectContext: { outcome: "completed", exitCode: 0 },
    },
  ],
  "无超时不升级": [
    {
      name: "未提供超时时拒绝升级路径（r1428/r1436）",
      events: [
        { type: "spawn", maxBytes: 1024 },
        { type: "stdout", bytes: 10, lines: 1 },
        { type: "timeoutSigterm" },
      ],
      expectStates: ["running"],
      expectContext: { escalationRefused: 1, timeoutArmed: false },
    },
    {
      name: "随后仍可正常完成并记录真实退出码",
      events: [{ type: "finish", exitCode: 3 }],
      expectStates: ["finished"],
      expectContext: { outcome: "completed", exitCode: 3 },
    },
  ],
  "未超限忽略超限信号": [
    {
      name: "未超限时 outputLimitExceeded 不杀进程只计数",
      events: [
        { type: "spawn", maxBytes: 1024 },
        { type: "stdout", bytes: 10, lines: 1 },
        { type: "outputLimitExceeded" },
      ],
      expectStates: ["running"],
      expectContext: { ignoredLimitSignals: 1 },
    },
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
