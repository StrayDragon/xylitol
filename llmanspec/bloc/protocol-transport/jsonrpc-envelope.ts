/**
 * JSON-RPC 2.0 文本信封状态机（JSON-RPC Envelope）。
 *
 * 业务：client 与 host 之间的产品消息必须且仅须经 JSON-RPC 2.0 文本信封投递。
 * 客户端先经 host.describe 握手，对 protocol 整数与 formats（含 jsonrpc）协商；
 * protocol 或 formats 不匹配须致命断开，不得降级猜测，不得重试风暴。
 * 握手完成后按方法名寻址分发 unary；未登记方法（含预留前缀 plugin/*）以
 * -32601/unregistered_method 拒绝；非法信封以 -32700（解析错误）与 -32600
 * （非法请求、空 batch、batch 超限）拒绝；业务错误走 -32000 区间并在
 * error.data.code 携带稳定字符串。
 *
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r2917（信封形状：id 原样回显、通知无应答、batch 上限 100）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r2918（错误码与业务码：标准类别映射与业务区间稳定码）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r2919（方法名寻址：未登记 -32601，plugin 前缀预留未登记）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1704（握手与版本：protocol/formats 不匹配致命断开，不得降级）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1705（闭集缺口禁旁路：不得另开 REST 产品动词或第二套词表）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1706（审批问卷首应答：下行 notification 与客户端 unary 的信封载体）
 *
 * 实现落点：crates/xylitol-protocol/src/message.rs、crates/xylitol-protocol/src/codec.rs、
 * crates/xylitol-protocol/src/registry.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 线上协议版本：与 crates/xylitol-protocol/src/message.rs 的 PROTOCOL_VERSION 对齐。 */
export const PROTOCOL_VERSION = 7;

/** batch 条目数上限：超出判非法请求（-32600，data.code=batch_too_large）。 */
export const MAX_BATCH_ITEMS = 100;

/** 预留命名空间前缀：当前未登记，调用须返回 -32601。 */
export const RESERVED_METHOD_PREFIX = "plugin/";

/** 产品方法表抽样快照（REGISTRY 抽样）：方法名寻址，不引入数字 method_id。 */
export const REGISTERED_METHODS: readonly string[] = [
  "host.describe",
  "prompt",
  "abort",
  "subscribe",
  "approve_tool",
  "answer_question",
  "switch_session",
  "get_messages",
];

/** 方法是否已登记：预留前缀一律未登记（r2919）。 */
function isRegisteredMethod(method: string): boolean {
  if (method.startsWith(RESERVED_METHOD_PREFIX)) return false;
  return REGISTERED_METHODS.includes(method);
}

/** 信封会话上下文：只记录握手事实、分发计数与最近一次拒绝。 */
export interface EnvelopeContext {
  /** 握手成功的协议版本；未握手为 null。 */
  protocolVersion: number | null;
  /** 握手 formats 是否含 jsonrpc。 */
  formatsJsonrpc: boolean;
  /** 握手后已分发的 unary 请求数。 */
  dispatched: number;
  /** 不执行、不应答转交的上行 notification 数。 */
  notifications: number;
  /** 以未登记方法（-32601）拒绝的次数。 */
  unregisteredCount: number;
  /** 非法信封拒绝次数（-32700 / -32600）。 */
  malformedCount: number;
  /** 最近一次拒绝的 JSON-RPC 错误码。 */
  lastErrorCode: number | null;
  /** 最近一次拒绝的稳定业务码（error.data.code）。 */
  lastErrorDataCode: string | null;
  /** 最近一次原样回显的请求 id（r2917）。 */
  lastEchoedId: string | number | null;
}

export const initialContext: EnvelopeContext = {
  protocolVersion: null,
  formatsJsonrpc: false,
  dispatched: 0,
  notifications: 0,
  unregisteredCount: 0,
  malformedCount: 0,
  lastErrorCode: null,
  lastErrorDataCode: null,
  lastEchoedId: null,
};

export const meta = defineMeta({
  name: "jsonrpc-envelope",
  pkg: "protocol-transport",
  purpose: "JSON-RPC 2.0 文本信封：握手协商、方法名寻址、标准码与业务码拒绝",
  anchors: [
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r2917",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r2918",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r2919",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1704",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1705",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1706",
  ],
  impl: [
    "crates/xylitol-protocol/src/message.rs",
    "crates/xylitol-protocol/src/codec.rs",
    "crates/xylitol-protocol/src/registry.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as EnvelopeContext,
    events: {} as
      | { type: "describeResult"; protocol: number; formats: readonly string[] }
      | { type: "request"; method: string; id: string | number }
      | { type: "notification"; method: string }
      | {
          type: "malformedEnvelope";
          kind: "parse-error" | "invalid-request" | "empty-batch" | "batch-too-large";
        }
      | { type: "close" },
  },
  guards: {
    versionAndFormatsCompatible: ({ event }) =>
      event.type === "describeResult" &&
      event.protocol === PROTOCOL_VERSION &&
      event.formats.includes("jsonrpc"),
    methodRegistered: ({ event }) => event.type === "request" && isRegisteredMethod(event.method),
  },
  actions: {
    recordHandshake: assign({
      protocolVersion: ({ event }) => (event.type === "describeResult" ? event.protocol : null),
      formatsJsonrpc: ({ event }) =>
        event.type === "describeResult" ? event.formats.includes("jsonrpc") : false,
    }),
    recordHandshakeMismatch: assign({
      lastErrorCode: () => -32000,
      lastErrorDataCode: () => "protocol_mismatch",
    }),
    dispatchUnary: assign({
      dispatched: ({ context }) => context.dispatched + 1,
      lastEchoedId: ({ event }) => (event.type === "request" ? event.id : null),
    }),
    replyUnregistered: assign({
      unregisteredCount: ({ context }) => context.unregisteredCount + 1,
      lastErrorCode: () => -32601,
      lastErrorDataCode: () => "unregistered_method",
    }),
    forwardNotification: assign({
      notifications: ({ context }) => context.notifications + 1,
    }),
    replyMalformed: assign({
      malformedCount: ({ context }) => context.malformedCount + 1,
      lastErrorCode: ({ event }) =>
        event.type === "malformedEnvelope" ? (event.kind === "parse-error" ? -32700 : -32600) : null,
      lastErrorDataCode: ({ event }): string | null => {
        if (event.type !== "malformedEnvelope") return null;
        if (event.kind === "parse-error") return "parse_error";
        if (event.kind === "batch-too-large") return "batch_too_large";
        return "invalid_request";
      },
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "jsonrpc-envelope",
  initial: "handshaking",
  context: initialContext,
  states: {
    handshaking: {
      on: {
        describeResult: [
          { guard: "versionAndFormatsCompatible", target: "open", actions: "recordHandshake" },
          { target: "handshakeFailed", actions: "recordHandshakeMismatch" },
        ],
      },
    },
    open: {
      on: {
        request: [
          { guard: "methodRegistered", actions: "dispatchUnary" },
          { actions: "replyUnregistered" },
        ],
        notification: { actions: "forwardNotification" },
        malformedEnvelope: { actions: "replyMalformed" },
        close: { target: "closed" },
      },
    },
    handshakeFailed: { type: "final" },
    closed: { type: "final" },
  },
});

export const scenarioGroups = {
  "握手成功后按方法名分发": [
    { name: "host.describe 协商通过", events: [{ type: "describeResult", protocol: PROTOCOL_VERSION, formats: ["jsonrpc"] }], expectStates: ["open"], expectContext: { protocolVersion: PROTOCOL_VERSION, formatsJsonrpc: true } },
    { name: "已登记 unary 分发并回显数字 id", events: [{ type: "request", method: "host.describe", id: 1 }], expectStates: ["open"], expectContext: { dispatched: 1, lastEchoedId: 1 } },
    { name: "字符串 id 同样原样回显", events: [{ type: "request", method: "get_messages", id: "abc-1" }], expectStates: ["open"], expectContext: { dispatched: 2, lastEchoedId: "abc-1" } },
    { name: "上行 notification 不执行不应答", events: [{ type: "notification", method: "session/event" }], expectStates: ["open"], expectContext: { notifications: 1, dispatched: 2 } },
  ],
  "版本不匹配致命断开": [
    { name: "protocol 低于声明版本即断开", events: [{ type: "describeResult", protocol: PROTOCOL_VERSION - 1, formats: ["jsonrpc"] }], expectStates: ["handshakeFailed"], expectContext: { lastErrorCode: -32000, lastErrorDataCode: "protocol_mismatch", dispatched: 0 } },
    { name: "断开后请求不再受理", events: [{ type: "request", method: "prompt", id: 2 }], expectStates: ["handshakeFailed"], expectContext: { dispatched: 0 } },
  ],
  "格式不匹配致命断开": [
    { name: "formats 缺 jsonrpc 即断开", events: [{ type: "describeResult", protocol: PROTOCOL_VERSION, formats: ["rest"] }], expectStates: ["handshakeFailed"], expectContext: { formatsJsonrpc: false, lastErrorCode: -32000, dispatched: 0 } },
  ],
  "未登记方法与预留前缀拒绝": [
    { name: "握手进入分发态", events: [{ type: "describeResult", protocol: PROTOCOL_VERSION, formats: ["jsonrpc"] }], expectStates: ["open"] },
    { name: "未知方法名以 -32601 拒绝", events: [{ type: "request", method: "no_such_method", id: 9 }], expectStates: ["open"], expectContext: { unregisteredCount: 1, lastErrorCode: -32601, lastErrorDataCode: "unregistered_method" } },
    { name: "预留前缀 plugin 同样未登记", events: [{ type: "request", method: "plugin/call", id: 10 }], expectStates: ["open"], expectContext: { unregisteredCount: 2 } },
  ],
  "非法信封按标准码拒绝": [
    { name: "握手进入分发态", events: [{ type: "describeResult", protocol: PROTOCOL_VERSION, formats: ["jsonrpc"] }], expectStates: ["open"] },
    { name: "解析错误判 -32700", events: [{ type: "malformedEnvelope", kind: "parse-error" }], expectContext: { malformedCount: 1, lastErrorCode: -32700, lastErrorDataCode: "parse_error" } },
    { name: "空 batch 判 -32600", events: [{ type: "malformedEnvelope", kind: "empty-batch" }], expectContext: { malformedCount: 2, lastErrorCode: -32600, lastErrorDataCode: "invalid_request" } },
    { name: "batch 超限判 -32600 且 data.code 为 batch_too_large", events: [{ type: "malformedEnvelope", kind: "batch-too-large" }], expectContext: { malformedCount: 3, lastErrorCode: -32600, lastErrorDataCode: "batch_too_large" } },
  ],
  "握手完成前不分发": [
    { name: "未握手收到 unary 不受理", events: [{ type: "request", method: "host.describe", id: 1 }], expectStates: ["handshaking"], expectContext: { dispatched: 0, lastEchoedId: null } },
    { name: "协商失败后仍未分发", events: [{ type: "describeResult", protocol: 99, formats: ["jsonrpc"] }, { type: "request", method: "prompt", id: 3 }], expectStates: ["handshakeFailed"], expectContext: { dispatched: 0 } },
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
