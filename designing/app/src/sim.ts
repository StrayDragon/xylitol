import type { Span } from "./types";

/** Interactive lab prototype contract (tui-lab modules only). */
export type SimLines = Span[][];

export interface Sim {
  /** 键位提示，每行一条，渲染在终端下方 */
  hint: string[];
  /** 可选自动心跳（ms）；每次心跳派发一次 onKey("__tick__") */
  tickMs?: number;
  initial(): unknown;
  /** key = 浏览器 KeyboardEvent.key（单字符 / "ArrowLeft" / "__tick__"） */
  onKey(key: string, model: unknown): unknown;
  view(model: unknown): SimLines;
}
