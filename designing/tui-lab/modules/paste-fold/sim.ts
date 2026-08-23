import type { Sim } from "../../../app/src/sim";
import type { Span } from "../../../app/src/types";

/** paste-fold 交互原型：验证「粘贴块作为一个单元」的光标语义。 */
type Seg = { kind: "text"; text: string } | { kind: "paste"; lines: number };

type Model = {
  segs: Seg[];
  /** 光标所在 seg 下标 + seg 内逻辑偏移 */
  idx: number;
  off: number;
  /** true = 提案行为（整块单元）；false = 现状对照（逐字符穿过展开文本） */
  unit: boolean;
  presses: number;
  flash: string;
};

const PASTE_LINES = 12;
/** plain 模式下展开粘贴文本的可见长度（模拟真实成本） */
const PLAIN_LEN = 46;
const EXPANDED =
  "def handle(err):    if err is None:        return None    log = open('h.log')    …";

function effLen(m: Model, s: Seg): number {
  if (s.kind === "text") return s.text.length;
  return m.unit ? 1 : PLAIN_LEN;
}

function flatPos(m: Model): number {
  let n = 0;
  for (let i = 0; i < m.idx && i < m.segs.length; i++) n += effLen(m, m.segs[i]);
  const cur = m.segs[m.idx];
  return n + (cur ? Math.min(m.off, effLen(m, cur)) : 0);
}

function totalLen(m: Model): number {
  return m.segs.reduce((n, s) => n + effLen(m, s), 0);
}

function locate(m: Model, pos: number): void {
  let rest = Math.max(0, pos);
  m.idx = m.segs.length;
  m.off = 0;
  for (let i = 0; i < m.segs.length; i++) {
    const L = effLen(m, m.segs[i]);
    if (rest <= L) {
      m.idx = i;
      m.off = rest;
      return;
    }
    rest -= L;
  }
}

function move(m: Model, dir: number): void {
  locate(m, flatPos(m) + dir);
  m.presses++;
}

export const sim: Sim = {
  hint: [
    "← → 移动光标（unit 模式整块跳过）",
    "p 插入示例粘贴块",
    "t 切换 unit/plain 对照",
    "Backspace 删除",
    "Enter 查看提交语义",
  ],
  initial(): Model {
    return {
      segs: [{ kind: "text", text: "帮我参考这个实现：" }],
      idx: 1,
      off: 0,
      unit: true,
      presses: 0,
      flash: "按 p 把一段 12 行代码粘进提示词，再用 ← → 感受两种模式。",
    };
  },
  onKey(key, raw): unknown {
    const m = raw as Model;
    m.flash = "";
    if (key === "ArrowLeft") move(m, -1);
    else if (key === "ArrowRight") move(m, 1);
    else if (key === "t") {
      const pos = flatPos(m);
      m.unit = !m.unit;
      locate(m, Math.min(pos, totalLen(m)));
      m.flash = m.unit ? "unit：提案行为——粘贴块是一个单元" : "plain：现状对照——逐字符穿过展开全文";
    } else if (key === "p") {
      m.segs.splice(m.idx, 0, { kind: "paste", lines: PASTE_LINES });
      locate(m, flatPos(m) + (m.unit ? 1 : PLAIN_LEN));
    } else if (key === "Backspace") {
      if (flatPos(m) === 0) return m;
      const cur = m.segs[m.idx];
      if (cur && cur.kind === "paste" && m.off > 0 && !m.unit) {
        // plain：从展开文本里删一个字符（长度收缩）
        return m;
      }
      const prev = m.segs[m.idx - 1];
      if (prev?.kind === "paste") {
        m.segs.splice(m.idx - 1, 1);
        m.idx = Math.max(0, m.idx - 1);
        m.off = 0;
      } else if (cur?.kind === "text" && m.off > 0) {
        cur.text = cur.text.slice(0, m.off - 1) + cur.text.slice(m.off);
        m.off--;
      } else if (m.idx > 0 && m.segs[m.idx - 1]?.kind === "text") {
        const t = m.segs[m.idx - 1] as { kind: "text"; text: string };
        t.text = t.text.slice(0, -1);
        m.idx--;
        m.off = t.text.length;
      } else if (cur?.kind === "paste") {
        // 行首遇到粘贴块（unit）：删整块
        m.segs.splice(m.idx, 1);
      }
      m.presses++;
    } else if (key === "Enter") {
      m.flash = m.unit
        ? "↵ 提交语义不变：模型收到完整原文（折叠只是显示方式）"
        : "↵ 提交语义不变：缓冲里本来就是全量原文";
    } else if (key.length === 1 && key !== " ") {
      const cur = m.segs[m.idx] as { kind: "text"; text: string } | undefined;
      if (cur?.kind === "text") {
        cur.text = cur.text.slice(0, m.off) + key + cur.text.slice(m.off);
        m.off++;
      } else {
        m.segs.splice(m.idx, 0, { kind: "text", text: key });
        m.idx++;
        m.off = 0;
      }
    }
    return m;
  },
  view(raw): Span[][] {
    const m = raw as Model;
    const mode: Span = {
      text: `[${m.unit ? "unit 整块单元" : "plain 现状对照"}] · 光标移动按键 ${m.presses}`,
      token: "muted",
    };
    const editorRow: Span[] = [{ text: "❯ ", token: "accent" }];
    const cursorIn = (ch: string): Span => ({ text: ch || " ", token: "on-surface", rev: true });
    let placed = false;
    for (let i = 0; i <= m.segs.length; i++) {
      if (i === m.idx) {
        editorRow.push(cursorIn(" "));
        placed = true;
      }
      const s = m.segs[i];
      if (!s) continue;
      if (s.kind === "text") {
        if (m.idx === i && !placed) {
          editorRow.push({ text: s.text.slice(0, m.off), token: "on-surface" });
          editorRow.push(cursorIn(s.text[m.off] ?? " "));
          editorRow.push({ text: s.text.slice(m.off + 1), token: "on-surface" });
          placed = true;
        } else {
          editorRow.push({ text: s.text, token: "on-surface" });
        }
      } else if (m.unit) {
        editorRow.push({ text: `[Pasted ~${s.lines} lines]`, token: "skill-ref" });
      } else {
        const head = i < m.idx ? PLAIN_LEN : i === m.idx ? m.off : 0;
        const visible = EXPANDED.slice(0, Math.min(head, EXPANDED.length));
        editorRow.push({ text: `「${visible}`, token: "muted" });
        if (i === m.idx) {
          editorRow.push(cursorIn(EXPANDED[m.off] ?? " "));
          editorRow.push({
            text: `${EXPANDED.slice(m.off + 1, PLAIN_LEN)}…」`,
            token: "muted",
          });
          placed = true;
        } else {
          editorRow.push({ text: "…」", token: "muted" });
        }
      }
    }
    const rows: Span[][] = [
      [{ text: "─".repeat(60), token: "muted" }],
      editorRow,
      [{ text: "─".repeat(60), token: "muted" }],
      [mode],
    ];
    if (m.flash) rows.push([{ text: m.flash, token: "accent" }]);
    return rows;
  },
};
