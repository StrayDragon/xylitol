/**
 * Provider 流式终态状态机（Provider Stream Terminal）。
 *
 * 业务：SSE 帧驱动的流式生成：终态约束（Done 恰一次且最后、每个
 * ToolCallStart 恰配一个 ToolCallEnd、异常 EOF 必须报错、中断时补空参
 * ToolCallEnd）；SSE error 事件转结构化 provider 错误；坏帧连续 3 帧终断、
 * 散发坏帧跳过续流；abort 时按观测档 finalize（ERROR/aborted，不伪造
 * usage）；stop_reason 完整映射终态分类。
 *
 * @spec llmanspec/specs/infra-provider/infra-provider.feature#@req:r3308（chunk 流终态约束）
 * @spec llmanspec/specs/infra-provider/infra-provider.feature#@req:r3012（SSE error 转错误、message_stop 兜底 Done、异常 EOF）
 * @spec llmanspec/specs/infra-provider/infra-provider.feature#@req:r3013（坏帧计数告警，连续 3 帧终断）
 * @spec llmanspec/specs/infra-provider/infra-provider.feature#@req:r3022（结构化 provider 错误）
 * @spec llmanspec/specs/infra-provider/infra-provider.feature#@req:r3354（stop_reason 完整映射）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1474（abort finalize：ERROR/aborted，不伪造 usage）
 *
 * 实现落点：plugins/xylitol-official-plugin-provider-anthropic/src/provider/（SSE 解析与错误结构）、
 * plugins/xylitol-official-plugin-provider-openai/src/provider/（同终态矩阵的另一方言）。
 *
 * 建模说明：guard 在动作执行前求值——corruptAtThird 用 streak+1 自行推算
 * 本帧是否为连续第 3 帧坏帧；工具结束配对、重复 Done、Done 后再产出均建模
 * 为可观测的违约转移。r1474 的观测 flush 只在 observationIo 非 none 时置位。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type ObservationIo = "none" | "truncated" | "full";

/** 终态分类：由 stop_reason 映射（r3354）。 */
export type StopClassification =
  | "completed"
  | "length-truncated"
  | "tool-call"
  | "pause-turn"
  | null;

/** 映射为错误的 stop_reason（r3354：refusal / sensitive）。 */
export const STOP_ERROR_REASONS = ["refusal", "sensitive"] as const;

/** 连续坏帧终断阈值（r3013）。 */
export const CORRUPT_FRAME_LIMIT = 3;

export function classifyStop(reason: string | null): StopClassification {
  if (reason === null) return "completed";
  if (reason === "max_tokens" || reason === "model_context_window_exceeded") {
    return "length-truncated";
  }
  if (reason === "tool_use") return "tool-call";
  if (reason === "pause_turn") return "pause-turn";
  return "completed";
}

export interface OpenEvent {
  type: "open";
  observationIo: ObservationIo;
  bufferedInput: boolean;
}

export interface SseErrorEvent {
  type: "sseError";
  /** SSE error 事件的 error.type，进入结构化错误的 code 位。 */
  errorType: string;
  message: string;
  httpStatus?: number;
  retryAfterSec?: string;
}

export interface StructuredProviderError {
  kind: string;
  httpStatus: number | null;
  code: string | null;
  retryAfterSec: string | null;
  message: string;
}

export interface StreamContext {
  observationIo: ObservationIo;
  bufferedInput: boolean;
  doneEmitted: boolean;
  openToolCalls: number;
  toolCallEnds: number;
  paddedToolEnds: number;
  corruptStreak: number;
  corruptTotal: number;
  stopReason: string | null;
  classification: StopClassification;
  error: StructuredProviderError | null;
  finalizeLevel: string | null;
  finalizeMessage: string | null;
  flushedOnAbort: boolean;
  usageFabricated: boolean;
}

export const initialContext: StreamContext = {
  observationIo: "none",
  bufferedInput: false,
  doneEmitted: false,
  openToolCalls: 0,
  toolCallEnds: 0,
  paddedToolEnds: 0,
  corruptStreak: 0,
  corruptTotal: 0,
  stopReason: null,
  classification: null,
  error: null,
  finalizeLevel: null,
  finalizeMessage: null,
  flushedOnAbort: false,
  usageFabricated: false,
};

export const meta = defineMeta({
  name: "provider-stream-terminal",
  pkg: "security-infra",
  purpose: "Provider SSE 流终态约束：Done 唯一、坏帧守卫、结构化错误、abort finalize、stop_reason 映射",
  anchors: [
    "llmanspec/specs/infra-provider/infra-provider.feature#@req:r3308",
    "llmanspec/specs/infra-provider/infra-provider.feature#@req:r3012",
    "llmanspec/specs/infra-provider/infra-provider.feature#@req:r3013",
    "llmanspec/specs/infra-provider/infra-provider.feature#@req:r3022",
    "llmanspec/specs/infra-provider/infra-provider.feature#@req:r3354",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1474",
  ],
  impl: [
    "plugins/xylitol-official-plugin-provider-anthropic/src/provider/",
    "plugins/xylitol-official-plugin-provider-openai/src/provider/",
  ],
});

function providerError(kind: string, message: string): StructuredProviderError {
  return { kind, httpStatus: null, code: null, retryAfterSec: null, message };
}

export const setupMachine = setup({
  types: {
    context: {} as StreamContext,
    events: {} as
      | OpenEvent
      | { type: "frame"; kind: "text" | "thinking" }
      | { type: "toolStart" }
      | { type: "toolEnd" }
      | { type: "stopReason"; reason: string }
      | { type: "messageStop" }
      | SseErrorEvent
      | { type: "corruptFrame" }
      | { type: "eof" }
      | { type: "abort" },
  },
  guards: {
    hasOpenTool: ({ context }) => context.openToolCalls > 0,
    stopReasonIsError: ({ context }) =>
        (STOP_ERROR_REASONS as readonly string[]).includes(context.stopReason ?? ""),
    // guard 先于动作求值：本帧若计入后达到 3，则在此帧终断（r3013）。
    corruptAtThird: ({ context }) => context.corruptStreak + 1 >= CORRUPT_FRAME_LIMIT,
  },
  actions: {
    beginStream: assign(({ event }) => {
      const e = event as OpenEvent;
      return { observationIo: e.observationIo, bufferedInput: e.bufferedInput };
    }),
    resetStreak: assign({ corruptStreak: () => 0 }),
    openTool: assign(({ context }) => ({
      openToolCalls: context.openToolCalls + 1,
    })),
    closeTool: assign(({ context }) => ({
      openToolCalls: context.openToolCalls - 1,
      toolCallEnds: context.toolCallEnds + 1,
    })),
    orphanToolEnd: assign(({ context }) => ({
      error: providerError("orphan-tool-end", "toolEnd 无配对 toolStart"),
      corruptStreak: context.corruptStreak,
    })),
    setStopReason: assign(({ event }) => ({
      stopReason: (event as { reason: string }).reason,
    })),
    finishDone: assign(({ context }) => ({
      doneEmitted: true,
      classification: classifyStop(context.stopReason),
    })),
    failStopReasonError: assign(({ context }) => ({
      error: providerError("stop-reason-error:" + (context.stopReason ?? ""), "stop_reason 映射为错误（r3354）"),
    })),
    recordProviderError: assign(({ event }) => {
      const e = event as SseErrorEvent;
      return {
        error: {
          kind: "provider",
          httpStatus: e.httpStatus ?? null,
          code: e.errorType,
          retryAfterSec: e.retryAfterSec ?? null,
          message: e.message,
        },
      };
    }),
    countCorrupt: assign(({ context }) => ({
      corruptStreak: context.corruptStreak + 1,
      corruptTotal: context.corruptTotal + 1,
    })),
    failCorruptGuard: assign(({ context }) => ({
      corruptStreak: context.corruptStreak + 1,
      corruptTotal: context.corruptTotal + 1,
      error: providerError("provider-corrupt-frame", `连续 ${CORRUPT_FRAME_LIMIT} 帧解析失败，终断流（r3013）`),
    })),
    padOpenToolEnd: assign(({ context }) => ({
      openToolCalls: 0,
      toolCallEnds: context.toolCallEnds + 1,
      paddedToolEnds: context.paddedToolEnds + 1,
    })),
    failAbnormalEof: assign({
      error: () => providerError("provider-abnormal-eof", "EOF 未见终止事件，不得静默当作完成（r3012/r3308）"),
    }),
    failPostDoneChunk: assign({
      error: () => providerError("post-done-chunk", "Done 之后不得再产出 chunk（r3308）"),
    }),
    failDuplicateDone: assign({
      error: () => providerError("duplicate-done", "Done 不得重复发出（r3012/r3308）"),
    }),
    finalizeAbort: assign(({ context }) => ({
      // r1474：observationIo 非 none 才按档 flush 已缓冲输入输出。
      flushedOnAbort: context.observationIo !== "none" && context.bufferedInput,
      finalizeLevel: "ERROR",
      finalizeMessage: "aborted",
      usageFabricated: false,
    })),
  },
});

export const machine = setupMachine.createMachine({
  id: "provider-stream-terminal",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        open: { target: "streaming", actions: "beginStream" },
      },
    },
    streaming: {
      on: {
        frame: { actions: "resetStreak" },
        toolStart: { actions: ["resetStreak", "openTool"] },
        toolEnd: [
          { guard: "hasOpenTool", actions: ["resetStreak", "closeTool"] },
          { target: "failed", actions: "orphanToolEnd" },
        ],
        stopReason: { actions: ["resetStreak", "setStopReason"] },
        messageStop: [
          { guard: "stopReasonIsError", target: "failed", actions: "failStopReasonError" },
          { target: "done", actions: "finishDone" },
        ],
        // r3012/r3022：SSE error 事件转结构化错误，不静默结束流。
        sseError: { target: "failed", actions: "recordProviderError" },
        corruptFrame: [
          { guard: "corruptAtThird", target: "failed", actions: "failCorruptGuard" },
          { actions: "countCorrupt" },
        ],
        eof: [
          // r3308：中断时已开 ToolCall 必须补空参 ToolCallEnd 再报错。
          { guard: "hasOpenTool", target: "failed", actions: ["padOpenToolEnd", "failAbnormalEof"] },
          { target: "failed", actions: "failAbnormalEof" },
        ],
        // r1474：未见成功 Done 提前结束（用户 abort）时按档 finalize。
        abort: { target: "aborted", actions: "finalizeAbort" },
      },
    },
    done: {
      on: {
        frame: { target: "failed", actions: "failPostDoneChunk" },
        toolStart: { target: "failed", actions: "failPostDoneChunk" },
        messageStop: { target: "failed", actions: "failDuplicateDone" },
      },
    },
    failed: { type: "final" },
    aborted: { type: "final" },
  },
});

export const scenarioGroups = {
  "正常终态一次且最后": [
    {
      name: "文本与工具调用帧正常流动",
      events: [
        { type: "open", observationIo: "truncated", bufferedInput: true },
        { type: "frame", kind: "text" },
        { type: "toolStart" },
        { type: "toolEnd" },
        { type: "stopReason", reason: "tool_use" },
      ],
      expectStates: ["streaming"],
      expectContext: { openToolCalls: 0, toolCallEnds: 1, corruptStreak: 0 },
    },
    {
      name: "message_stop 兜底 Done 并映射 tool_use 分类（r3012/r3354）",
      events: [{ type: "messageStop" }],
      expectStates: ["done"],
      expectContext: { doneEmitted: true, classification: "tool-call" },
    },
  ],
  "Done 后产出即违约": [
    {
      name: "Done 后再产出 chunk 被校验器判违约（r3308 拒绝路径）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "messageStop" },
        { type: "frame", kind: "text" },
      ],
      expectStates: ["failed"],
      expect: (snapshot) => {
        const err = (snapshot as { context: StreamContext }).context.error;
        return err !== null && err.kind === "post-done-chunk"
            ? true
            : `违约类型不符：${JSON.stringify(err)}`;
      },
    },
  ],
  "重复 Done 违约": [
    {
      name: "message_stop 重复出现时判违约（r3012/r3308）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "messageStop" },
        { type: "messageStop" },
      ],
      expectStates: ["failed"],
      expect: (snapshot) => {
        const err = (snapshot as { context: StreamContext }).context.error;
        return err !== null && err.kind === "duplicate-done" ? true : "未判重复 Done";
      },
    },
  ],
  "异常 EOF 补空参工具结束": [
    {
      name: "已开 ToolCall 未收就 EOF：补空参 ToolCallEnd 再报错（r3308/r3012）",
      events: [
        { type: "open", observationIo: "full", bufferedInput: true },
        { type: "toolStart" },
        { type: "eof" },
      ],
      expectStates: ["failed"],
      expectContext: { paddedToolEnds: 1, toolCallEnds: 1, openToolCalls: 0, classification: null },
    },
  ],
  "SSE error 转结构化错误": [
    {
      name: "error 事件携带 type/message/status/retry-after（r3012/r3022）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        {
          type: "sseError",
          errorType: "overloaded_error",
          message: "Overloaded",
          httpStatus: 529,
          retryAfterSec: "2",
        },
      ],
      expectStates: ["failed"],
      expect: (snapshot) => {
        const err = (snapshot as { context: StreamContext }).context.error;
        return err !== null && err.kind === "provider" && err.httpStatus === 529 &&
                err.code === "overloaded_error" && err.retryAfterSec === "2"
            ? true
            : `结构化错误不完整：${JSON.stringify(err)}`;
      },
    },
  ],
  "连续三坏帧终断": [
    {
      name: "连续第 3 帧坏帧终断流（r3013 拒绝路径）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "corruptFrame" },
        { type: "corruptFrame" },
        { type: "corruptFrame" },
      ],
      expectStates: ["failed"],
      expectContext: { corruptStreak: 3, corruptTotal: 3 },
    },
  ],
  "散发坏帧跳过续流": [
    {
      name: "坏帧与好帧交替时计数并归零，流不中断（r3013）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "corruptFrame" },
        { type: "frame", kind: "text" },
        { type: "corruptFrame" },
        { type: "frame", kind: "thinking" },
        { type: "corruptFrame" },
        { type: "frame", kind: "text" },
      ],
      expectStates: ["streaming"],
      expectContext: { corruptStreak: 0, corruptTotal: 3, doneEmitted: false },
    },
  ],
  "abort finalize": [
    {
      name: "未见 Done 提前 abort，ERROR/aborted 收口并按档 flush（r1474）",
      events: [
        { type: "open", observationIo: "truncated", bufferedInput: true },
        { type: "frame", kind: "text" },
        { type: "abort" },
      ],
      expectStates: ["aborted"],
      expectContext: { flushedOnAbort: true, finalizeLevel: "ERROR", finalizeMessage: "aborted", usageFabricated: false, doneEmitted: false },
    },
  ],
  "abort 无观测不 flush": [
    {
      name: "observationIo=none 时不 flush，仍 ERROR/aborted（r1474）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "abort" },
      ],
      expectStates: ["aborted"],
      expectContext: { flushedOnAbort: false, finalizeLevel: "ERROR", finalizeMessage: "aborted" },
    },
  ],
  "refusal 映射错误": [
    {
      name: "stop_reason=refusal 映射为错误而非完成（r3354）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "stopReason", reason: "refusal" },
        { type: "messageStop" },
      ],
      expectStates: ["failed"],
      expect: (snapshot) => {
        const ctx = (snapshot as { context: StreamContext }).context;
        return ctx.error !== null && ctx.error.kind === "stop-reason-error:refusal" && ctx.doneEmitted === false
            ? true
            : "refusal 未按错误收口";
      },
    },
  ],
  "长度截断映射": [
    {
      name: "stop_reason=max_tokens 映射长度截断分类（r3354）",
      events: [
        { type: "open", observationIo: "none", bufferedInput: false },
        { type: "stopReason", reason: "max_tokens" },
        { type: "messageStop" },
      ],
      expectStates: ["done"],
      expectContext: { classification: "length-truncated", doneEmitted: true },
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
