/**
 * 远端审批与问卷请求状态机（Approval Request）。
 *
 * 业务：host 向 client 发起审批/问卷，经下行 notification 告知并挂起回合，
 * 匹配 call 的第一笔客户端 unary 应答生效，后续应答一律忽略。
 * 作答只允许人类操作（CLI/TUI 交互或已认证 client 的 unary），
 * 不得注册为 LLM 可调用工具，非人类来源的作答一律拒绝。
 * Choice 槽占用时新请求排队（容量 1）；当前 ask 完成后按序挂载排队项；
 * 队满须即时以拒绝载荷（approved=false）回应新请求并通知条提示、记日志，
 * 不得静默覆盖在等 ask 或丢弃其 oneshot。请求亦可超时或被撤销收口。
 *
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1706（审批问卷首应答：第一 unary 生效，后续忽略）
 * @spec llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r3327（远端审批队列：容量 1，队满即时拒绝）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3219（trust 决策只允许人类操作）
 *
 * 实现落点：crates/xylitol-serve/src/occupancy/host.rs（approval unary 首答生效）、
 * crates/xylitol-app-tui/src/layout/slots/ask.rs（Choice 槽排队）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 远端审批队列容量：Choice 槽占用时至多排队 1 笔（r3327）。 */
export const APPROVAL_QUEUE_CAPACITY = 1;

/** 审批请求上下文：槽占用、排队与终局事实。 */
export interface ApprovalContext {
  /** 当前挂起请求 id；无挂起为 null。 */
  requestId: string | null;
  /** 排队中的请求数（容量 1）。 */
  queued: number;
  /** 排队中的请求 id。 */
  queuedId: string | null;
  /** 队满即时拒绝次数（拒绝载荷 approved=false）。 */
  queueRejections: number;
  /** 队满拒绝伴随的通知条提示与日志次数。 */
  rejectionNotices: number;
  /** 最近一次队满拒绝是否使用 approved=false 载荷。 */
  queueRejectedApprovedFalse: boolean;
  /** 非人类来源作答被拒绝的次数（r3219）。 */
  rejectedNonHuman: number;
  /** 首答之后被忽略的迟到应答数（r1706）。 */
  lateAnswers: number;
  /** 终局结果。 */
  outcome: "approved" | "denied" | "timeout" | "revoked" | null;
}

export const initialContext: ApprovalContext = {
  requestId: null,
  queued: 0,
  queuedId: null,
  queueRejections: 0,
  rejectionNotices: 0,
  queueRejectedApprovedFalse: false,
  rejectedNonHuman: 0,
  lateAnswers: 0,
  outcome: null,
};

export const meta = defineMeta({
  name: "approval-request",
  pkg: "protocol-transport",
  purpose: "远端审批/问卷：首答生效、人类限定、槽占用排队与队满即时拒绝",
  anchors: [
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1706",
    "llmanspec/specs/app-tui-ask/app-tui-ask.feature#@req:r3327",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3219",
  ],
  impl: [
    "crates/xylitol-serve/src/occupancy/host.rs",
    "crates/xylitol-app-tui/src/layout/slots/ask.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as ApprovalContext,
    events: {} as
      | { type: "requestArrived"; requestId: string }
      | { type: "answer"; requestId: string; approved: boolean; source: "human" | "agent" }
      | { type: "requestTimeout" }
      | { type: "approvalRevoked" },
  },
  guards: {
    slotIdle: ({ context }) => context.requestId === null,
    queueHasRoom: ({ context }) => context.queued < APPROVAL_QUEUE_CAPACITY,
    nonHumanSource: ({ event }) => event.type === "answer" && event.source === "agent",
    hasQueued: ({ context }) => context.queued > 0,
  },
  actions: {
    mountRequest: assign({
      requestId: ({ event }) => (event.type === "requestArrived" ? event.requestId : null),
    }),
    enqueueRequest: assign({
      queued: ({ context }) => context.queued + 1,
      queuedId: ({ event }) => (event.type === "requestArrived" ? event.requestId : null),
    }),
    rejectQueuedRequest: assign({
      queueRejections: ({ context }) => context.queueRejections + 1,
      rejectionNotices: ({ context }) => context.rejectionNotices + 1,
      queueRejectedApprovedFalse: () => true,
    }),
    rejectNonHuman: assign({
      rejectedNonHuman: ({ context }) => context.rejectedNonHuman + 1,
    }),
    recordFirstAnswer: assign({
      outcome: ({ event }) =>
        event.type === "answer" ? (event.approved ? "approved" : "denied") : null,
    }),
    mountQueued: assign({
      requestId: ({ context }) => context.queuedId,
      queuedId: () => null,
      queued: ({ context }) => Math.max(0, context.queued - 1),
      outcome: () => null,
    }),
    recordTimeout: assign({
      outcome: () => "timeout" as const,
      requestId: () => null,
    }),
    recordRevoked: assign({
      outcome: () => "revoked" as const,
      requestId: () => null,
    }),
    countLateAnswer: assign({
      lateAnswers: ({ context }) => context.lateAnswers + 1,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "approval-request",
  initial: "awaiting",
  context: initialContext,
  states: {
    awaiting: {
      on: {
        requestArrived: [
          { guard: "slotIdle", actions: "mountRequest" },
          { guard: "queueHasRoom", actions: "enqueueRequest" },
          { actions: "rejectQueuedRequest" },
        ],
        answer: [
          { guard: "nonHumanSource", actions: "rejectNonHuman" },
          { guard: "hasQueued", target: "awaiting", actions: ["recordFirstAnswer", "mountQueued"] },
          { target: "resolved", actions: "recordFirstAnswer" },
        ],
        requestTimeout: { target: "resolved", actions: "recordTimeout" },
        approvalRevoked: { target: "resolved", actions: "recordRevoked" },
      },
    },
    resolved: {
      on: {
        answer: { actions: "countLateAnswer" },
        requestArrived: { target: "awaiting", actions: "mountRequest" },
      },
    },
  },
});

export const scenarioGroups = {
  "首答生效后续忽略": [
    { name: "审批请求挂起回合", events: [{ type: "requestArrived", requestId: "a1" }], expectStates: ["awaiting"], expectContext: { requestId: "a1", outcome: null } },
    { name: "第一笔人类 unary 批准生效", events: [{ type: "answer", requestId: "a1", approved: true, source: "human" }], expectStates: ["resolved"], expectContext: { outcome: "approved" } },
    { name: "后续应答被忽略且不翻转结果", events: [{ type: "answer", requestId: "a1", approved: false, source: "human" }], expectStates: ["resolved"], expectContext: { outcome: "approved", lateAnswers: 1 } },
  ],
  "非人类来源拒绝": [
    { name: "审批请求挂起", events: [{ type: "requestArrived", requestId: "a2" }], expectStates: ["awaiting"] },
    { name: "LLM 来源作答被拒绝（不得注册为工具）", events: [{ type: "answer", requestId: "a2", approved: true, source: "agent" }], expectStates: ["awaiting"], expectContext: { rejectedNonHuman: 1, outcome: null } },
    { name: "人类作答仍可生效", events: [{ type: "answer", requestId: "a2", approved: true, source: "human" }], expectStates: ["resolved"], expectContext: { outcome: "approved" } },
  ],
  "超时终局": [
    { name: "审批请求挂起", events: [{ type: "requestArrived", requestId: "a3" }], expectStates: ["awaiting"] },
    { name: "超时按拒绝收口", events: [{ type: "requestTimeout" }], expectStates: ["resolved"], expectContext: { outcome: "timeout", requestId: null } },
    { name: "迟到应答不得翻转终局", events: [{ type: "answer", requestId: "a3", approved: true, source: "human" }], expectStates: ["resolved"], expectContext: { outcome: "timeout", lateAnswers: 1 } },
  ],
  "撤销终局": [
    { name: "审批请求挂起", events: [{ type: "requestArrived", requestId: "a4" }], expectStates: ["awaiting"] },
    { name: "host 撤销请求", events: [{ type: "approvalRevoked" }], expectStates: ["resolved"], expectContext: { outcome: "revoked" } },
  ],
  "槽占用排队与队满即时拒绝": [
    { name: "首笔审批占用 Choice 槽", events: [{ type: "requestArrived", requestId: "a1" }], expectStates: ["awaiting"], expectContext: { requestId: "a1", queued: 0 } },
    { name: "第二笔进入容量 1 队列", events: [{ type: "requestArrived", requestId: "b1" }], expectStates: ["awaiting"], expectContext: { queued: 1, queuedId: "b1" } },
    { name: "队满即时拒绝第三笔（approved=false 载荷）", events: [{ type: "requestArrived", requestId: "c1" }], expectStates: ["awaiting"], expectContext: { queueRejections: 1, rejectionNotices: 1, queueRejectedApprovedFalse: true, queued: 1, queuedId: "b1" } },
    { name: "当前 ask 完成后按序挂载排队项", events: [{ type: "answer", requestId: "a1", approved: true, source: "human" }], expectStates: ["awaiting"], expectContext: { requestId: "b1", queued: 0, outcome: null } },
    { name: "排队项独立应答拒绝", events: [{ type: "answer", requestId: "b1", approved: false, source: "human" }], expectStates: ["resolved"], expectContext: { outcome: "denied" } },
  ],
  "人类拒绝路径": [
    { name: "审批请求挂起", events: [{ type: "requestArrived", requestId: "a5" }], expectStates: ["awaiting"] },
    { name: "人类明确拒绝生效", events: [{ type: "answer", requestId: "a5", approved: false, source: "human" }], expectStates: ["resolved"], expectContext: { outcome: "denied" } },
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
