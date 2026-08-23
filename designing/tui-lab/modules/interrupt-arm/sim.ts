import type { Sim } from "../../../app/src/sim";
import type { Span } from "../../../app/src/types";

/** interrupt-arm 交互原型：Esc 两段式中止——第一次出提示，5s 内再按才停。 */
type Phase = "busy" | "armed" | "stopped";

type Model = { phase: Phase; armedAt: number; spin: number; msg: string };

const FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const ARM_WINDOW = 5000;

export const sim: Sim = {
  tickMs: 90,
  hint: [
    "Esc 第一次出提示 / 第二次真正中止",
    "r 重置为进行中",
    "5 秒未确认自动解除",
  ],
  initial(): Model {
    return { phase: "busy", armedAt: 0, spin: 0, msg: "" };
  },
  onKey(key, raw): unknown {
    const m = raw as Model;
    if (key === "__tick__") {
      m.spin = (m.spin + 1) % FRAMES.length;
      if (m.phase === "armed" && Date.now() - m.armedAt > ARM_WINDOW) {
        m.phase = "busy";
        m.msg = "超时未确认，自动解除（任务继续）";
      }
      return m;
    }
    if (key === "Escape") {
      if (m.phase === "busy") {
        m.phase = "armed";
        m.armedAt = Date.now();
        m.msg = "";
      } else if (m.phase === "armed") {
        m.phase = "stopped";
        m.msg = "已中止当前任务（保留 partial，落盘 stop_reason=aborted）";
      }
    } else if (key === "r") {
      m.phase = "busy";
      m.msg = "";
    }
    return m;
  },
  view(raw): Span[][] {
    const m = raw as Model;
    const rows: Span[][] = [];
    if (m.phase === "stopped") {
      rows.push([{ text: "· Stopped", token: "muted" }]);
    } else {
      rows.push([
        { text: `${FRAMES[m.spin]} Working`, token: "accent" },
        { text: "  streaming…", token: "muted" },
      ]);
    }
    if (m.phase === "armed") {
      rows.push([{ text: "esc again to interrupt", token: "warning" }]);
    } else if (m.msg) {
      rows.push([{ text: m.msg, token: m.phase === "stopped" ? "muted" : "success" }]);
    }
    rows.push([{ text: "", token: "muted" }]);
    return rows;
  },
};
