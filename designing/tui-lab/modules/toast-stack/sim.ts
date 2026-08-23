import type { Sim } from "../../../app/src/sim";
import type { Span } from "../../../app/src/types";

/** toast-stack 交互原型：右上角通知栈——只收关键确认类通知。 */
type Note = { icon: "✓" | "◆"; text: string; action?: string; born: number };

type Model = { items: Note[]; flash: string; now: number };

const TTL = 4000;
const MAX = 3;

function push(m: Model, n: Omit<Note, "born">): void {
  m.items.unshift({ ...n, born: Date.now() });
  if (m.items.length > MAX) m.items.length = MAX;
}

export const sim: Sim = {
  tickMs: 250,
  hint: [
    "c 复制成功通知",
    "e 导出完成（带 › open 动作）",
    "Enter 执行最新动作",
    "d 关闭最新一条",
    "自动消失：4s",
  ],
  initial(): Model {
    return { items: [], flash: "按 c / e 弹出关键通知；右上角堆叠，4 秒自动消失。", now: 0 };
  },
  onKey(key, raw): unknown {
    const m = raw as Model;
    m.now = Date.now();
    if (key === "c") push(m, { icon: "✓", text: "Copied to clipboard" });
    else if (key === "e") push(m, { icon: "◆", text: "export done", action: "open" });
    else if (key === "d") {
      if (m.items.length) {
        m.items.shift();
        m.flash = "已关闭最新一条";
      }
    } else if (key === "Enter") {
      const top = m.items[0];
      m.flash = top?.action
        ? `执行动作：${top.action}（通知随即清除）`
        : "最新一条没有动作";
      if (top?.action) m.items.shift();
    } else if (key === "__tick__") {
      m.items = m.items.filter((n) => Date.now() - n.born < TTL);
    }
    return m;
  },
  view(raw): Span[][] {
    const m = raw as Model;
    const rows: Span[][] = [[{ text: "", token: "muted" }]];
    for (const n of m.items) {
      const content = `${n.icon} ${n.text}${n.action ? ` › ${n.action}` : ""}`;
      const pad = Math.max(0, 76 - content.length);
      rows.push([
        { text: " ".repeat(pad), token: "muted" },
        { text: content, token: n.icon === "✓" ? "success" : "accent" },
      ]);
    }
    if (!m.items.length) {
      rows.push([{ text: "                                     （无活动通知）", token: "muted" }]);
    }
    rows.push([{ text: "", token: "muted" }]);
    rows.push([{ text: `${m.items.length}/${MAX} · TTL 4s`, token: "muted" }]);
    if (m.flash) rows.push([{ text: m.flash, token: "accent" }]);
    return rows;
  },
};
