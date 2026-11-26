/**
 * 信任解析管线状态机（Trust Resolution Pipeline）。
 *
 * 业务：项目 trust 按固定优先级解析：显式覆盖 → 无信任输入自动信任 →
 * 带父继承的持久 store 查找 → UI 提示（仅 has_ui）→ 回退拒绝。
 * 解析同时产出决策与原因；store 篡改按未决处理，不静默信任（fail-closed）。
 *
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r73（解析管线固定优先级）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r72（store 持久化与父继承）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r71（trust 单一真源在 infra）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r74（UI 回调注入，不做终端 I/O）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r75（trust 决策接线项目 settings）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3216（store 加固，篡改按未决）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3218（advisory 边界）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3219（仅人类操作可触发）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3220（resume 按 workspace 重校验）
 * @spec llmanspec/specs/app-tui-trust/app-tui-trust.feature#@req:r3307（门禁答案 fail-closed）
 * @spec llmanspec/specs/app-tui-trust/app-tui-trust.feature#@req:r3223（仅人类确认 + tui 审计来源）
 *
 * 实现落点：crates/xylitol-runtime/src/trust/resolve.rs（TrustReason 与优先级）、
 * crates/xylitol-runtime/src/trust/store.rs（持久化、父继承与审计）。
 *
 * 建模说明：guard 按「动作执行前」的 context 求值；各闸口的输入字段由
 * captureInputs / beginRecheck 在上一步落盘，闸口 guard 只读 context。
 * 垃圾 prompt 答案（key 未命中或越界）按取消处理，不得默认放行（r3307）。
 * r74 的 store 持久化副作用不在本机建模，只记录审计来源与原因。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 解析原因：与 resolve.rs 的 TrustReason 枚举对齐。 */
export type TrustReason = "Override" | "NoTrustInputs" | "Store" | "UserPrompt" | "FallbackNoUi";

/** UI 提示的合法答案 key（稳定 key；未命中即垃圾答案，r3307）。 */
export const PROMPT_KEYS = ["trust", "deny"] as const;

export interface TrustResolutionEvent {
  type: "resolve";
  override: boolean | null;
  hasTrustInputs: boolean;
  storeDecision: boolean | null;
  storeTampered: boolean;
  hasUi: boolean;
  source?: string;
}

export interface PromptAnsweredEvent {
  type: "promptAnswered";
  key: string;
}

export interface RecheckEvent {
  type: "recheck";
  override: boolean | null;
  hasTrustInputs: boolean;
  storeDecision: boolean | null;
  storeTampered: boolean;
  hasUi: boolean;
}

export type TrustEvent =
  | TrustResolutionEvent
  | PromptAnsweredEvent
  | RecheckEvent
  | { type: "agentToolAttempt" };

export interface TrustContext {
  /** CLI 显式覆盖（--trust / --no-trust）；null 表示未提供。 */
  override: boolean | null;
  /** 目录是否存在 trust 所需输入（.xylitol/、.agents/skills/）。 */
  hasTrustInputs: boolean;
  /** 持久 store 决策（含父继承后的最近祖先值）；null 表示未决。 */
  storeDecision: boolean | null;
  /** store HMAC 校验失败；按未决处理并记篡改审计（r3216）。 */
  storeTampered: boolean;
  /** 是否存在可交互 UI；仅 has_ui 才允许提示（r73 步骤五）。 */
  hasUi: boolean;
  /** 本轮 prompt 收到的原始答案 key。 */
  promptKey: string | null;
  /** 解析出的 trust 决策；null 表示尚未解析。 */
  trusted: boolean | null;
  /** 解析原因，与 TrustReason 一一对应。 */
  reason: TrustReason | null;
  /** 审计事件流（事件:来源），对应 JSONL 审计语义。 */
  audit: string[];
  /** 触发来源（startup / resume / tui）。 */
  source: string;
  /** 非 LLM 工具触发被拒次数（r3219 / r3223）。 */
  agentTriggerRefusals: number;
}

export const initialContext: TrustContext = {
  override: null,
  hasTrustInputs: true,
  storeDecision: null,
  storeTampered: false,
  hasUi: true,
  promptKey: null,
  trusted: null,
  reason: null,
  audit: [],
  source: "startup",
  agentTriggerRefusals: 0,
};

export const meta = defineMeta({
  name: "trust-resolution",
  pkg: "security-infra",
  purpose: "项目信任按固定优先级解析：覆盖、无输入、store、UI 提示、回退拒绝",
  anchors: [
    "llmanspec/specs/domain-security/domain-security.feature#@req:r73",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r72",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r71",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r74",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r75",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3216",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3218",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3219",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3220",
    "llmanspec/specs/app-tui-trust/app-tui-trust.feature#@req:r3307",
    "llmanspec/specs/app-tui-trust/app-tui-trust.feature#@req:r3223",
  ],
  impl: ["crates/xylitol-runtime/src/trust/resolve.rs", "crates/xylitol-runtime/src/trust/store.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as TrustContext,
    events: {} as TrustEvent,
  },
  guards: {
    hasOverride: ({ context }) => context.override !== null,
    noTrustInputs: ({ context }) => !context.hasTrustInputs,
    storeTampered: ({ context }) => context.storeTampered,
    storeUsable: ({ context }) => !context.storeTampered && context.storeDecision !== null,
    uiUnavailable: ({ context }) => !context.hasUi,
    promptKeyValid: ({ event }) =>
      (PROMPT_KEYS as readonly string[]).includes((event as PromptAnsweredEvent).key),
  },
  actions: {
    captureInputs: assign(({ context, event }) => {
      const e = event as TrustResolutionEvent;
      return {
        override: e.override,
        hasTrustInputs: e.hasTrustInputs,
        storeDecision: e.storeDecision,
        storeTampered: e.storeTampered,
        hasUi: e.hasUi,
        promptKey: null,
        trusted: null,
        reason: null,
        source: e.source ?? context.source,
      };
    }),
    decideOverride: assign({
      trusted: ({ context }) => context.override === true,
      reason: () => "Override",
    }),
    decideNoInputs: assign({
      trusted: () => true,
      reason: () => "NoTrustInputs",
      audit: ({ context }) => [...context.audit, "trust_grant:no_trust_inputs"],
    }),
    decideStore: assign({
      trusted: ({ context }) => context.storeDecision === true,
      reason: () => "Store",
    }),
    recordTamper: assign({
      audit: ({ context }) => [...context.audit, "trust_tamper"],
    }),
    decideNoUi: assign({
      trusted: () => false,
      reason: () => "FallbackNoUi",
      audit: ({ context }) => [...context.audit, "trust_deny:no_ui"],
    }),
    decideUserPrompt: assign({
      trusted: ({ event }) => (event as PromptAnsweredEvent).key === "trust",
      reason: () => "UserPrompt",
      promptKey: ({ event }) => (event as PromptAnsweredEvent).key,
      source: () => "tui",
      audit: ({ context, event }) => [
        ...context.audit,
        `trust_${(event as PromptAnsweredEvent).key === "trust" ? "grant" : "deny"}:tui_prompt`,
      ],
    }),
    decideGarbageCancel: assign({
      trusted: () => false,
      reason: () => "FallbackNoUi",
      promptKey: ({ event }) => (event as PromptAnsweredEvent).key,
      audit: ({ context }) => [...context.audit, "trust_deny:prompt_cancelled"],
    }),
    beginRecheck: assign(({ context, event }) => {
      const e = event as RecheckEvent;
      return {
        override: e.override,
        hasTrustInputs: e.hasTrustInputs,
        storeDecision: e.storeDecision,
        storeTampered: e.storeTampered,
        hasUi: e.hasUi,
        promptKey: null,
        trusted: null,
        reason: null,
        source: "resume",
        audit: [...context.audit, "trust_recheck:resume"],
      };
    }),
    refuseAgentTrigger: assign({
      agentTriggerRefusals: ({ context }) => context.agentTriggerRefusals + 1,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "trust-resolution",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        resolve: { target: "overrideGate", actions: "captureInputs" },
        agentToolAttempt: { actions: "refuseAgentTrigger" },
      },
    },
    overrideGate: {
      always: [
        { guard: "hasOverride", target: "settled", actions: "decideOverride" },
        { target: "inputsGate" },
      ],
    },
    inputsGate: {
      always: [
        { guard: "noTrustInputs", target: "settled", actions: "decideNoInputs" },
        { target: "storeGate" },
      ],
    },
    storeGate: {
      always: [
        { guard: "storeTampered", target: "uiGate", actions: "recordTamper" },
        { guard: "storeUsable", target: "settled", actions: "decideStore" },
        { target: "uiGate" },
      ],
    },
    uiGate: {
      always: [
        { guard: "uiUnavailable", target: "settled", actions: "decideNoUi" },
        { target: "prompting" },
      ],
    },
    prompting: {
      on: {
        promptAnswered: [
          { guard: "promptKeyValid", target: "settled", actions: "decideUserPrompt" },
          { target: "settled", actions: "decideGarbageCancel" },
        ],
        agentToolAttempt: { actions: "refuseAgentTrigger" },
      },
    },
    settled: {
      on: {
        recheck: { target: "overrideGate", actions: "beginRecheck" },
        agentToolAttempt: { actions: "refuseAgentTrigger" },
      },
    },
  },
});

/** 断言审计流包含指定事件（expectContext 只做浅比较，含审计用此 helper）。 */
function auditContains(snapshot: unknown, needle: string): boolean | string {
  const audit = (snapshot as { context: TrustContext }).context.audit;
  return audit.includes(needle) ? true : `审计缺 ${needle}，实得 ${JSON.stringify(audit)}`;
}

export const scenarioGroups = {
  "覆盖胜出": [
    {
      name: "store 已标 untrusted 但显式覆盖为信任",
      events: [
        {
          type: "resolve",
          override: true,
          hasTrustInputs: true,
          storeDecision: false,
          storeTampered: false,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: true, reason: "Override" },
    },
  ],
  "覆盖显式拒绝": [
    {
      name: "--no-trust 覆盖即使 store 为 trusted 也拒绝",
      events: [
        {
          type: "resolve",
          override: false,
          hasTrustInputs: true,
          storeDecision: true,
          storeTampered: false,
          hasUi: true,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: false, reason: "Override" },
    },
  ],
  "无信任输入自动信任": [
    {
      name: "目录无 .xylitol/ 且无 .agents/skills/ 时自动信任",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: false,
          storeDecision: null,
          storeTampered: false,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: true, reason: "NoTrustInputs" },
    },
  ],
  "store 决策信任": [
    {
      name: "父继承后的 store 决策为 trusted",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: true,
          storeTampered: false,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: true, reason: "Store" },
    },
  ],
  "store 决策拒绝": [
    {
      name: "最近祖先标 untrusted 时子目录继承拒绝",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: false,
          storeTampered: false,
          hasUi: true,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: false, reason: "Store" },
    },
  ],
  "store 篡改按未决 fail-closed": [
    {
      name: "HMAC 失配不静默信任且记篡改审计",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: true,
          storeTampered: true,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: false, reason: "FallbackNoUi" },
      expect: (snapshot) => auditContains(snapshot, "trust_tamper"),
    },
  ],
  "无界面回退拒绝": [
    {
      name: "有输入、无 store 决策且无 UI 时回退拒绝",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: null,
          storeTampered: false,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: false, reason: "FallbackNoUi" },
      expect: (snapshot) => auditContains(snapshot, "trust_deny:no_ui"),
    },
  ],
  "UI 提示信任": [
    {
      name: "has_ui 时提示并选择 trust，审计来源为 tui",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: null,
          storeTampered: false,
          hasUi: true,
        },
        { type: "promptAnswered", key: "trust" },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: true, reason: "UserPrompt", source: "tui" },
      expect: (snapshot) => auditContains(snapshot, "trust_grant:tui_prompt"),
    },
  ],
  "垃圾答案取消": [
    {
      name: "key 未命中的垃圾答案按取消处理，不默认放行",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: null,
          storeTampered: false,
          hasUi: true,
        },
        { type: "promptAnswered", key: "garbage" },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: false, reason: "FallbackNoUi", promptKey: "garbage" },
      expect: (snapshot) => auditContains(snapshot, "trust_deny:prompt_cancelled"),
    },
  ],
  "resume 重校验": [
    {
      name: "启动时经 store 信任",
      events: [
        {
          type: "resolve",
          override: null,
          hasTrustInputs: true,
          storeDecision: true,
          storeTampered: false,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: true, reason: "Store" },
    },
    {
      name: "resume 换到 untrusted 工作区时重解析并记 trust_recheck",
      events: [
        {
          type: "recheck",
          override: null,
          hasTrustInputs: true,
          storeDecision: false,
          storeTampered: false,
          hasUi: false,
        },
      ],
      expectStates: ["settled"],
      expectContext: { trusted: false, reason: "Store", source: "resume" },
      expect: (snapshot) => auditContains(snapshot, "trust_recheck:resume"),
    },
  ],
  "agent 工具触发被拒": [
    {
      name: "LLM 可调用工具不得触发信任决策",
      events: [{ type: "agentToolAttempt" }],
      expectStates: ["idle"],
      expectContext: { agentTriggerRefusals: 1, trusted: null },
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
