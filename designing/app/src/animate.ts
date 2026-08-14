import type { Span, StateDoc } from "./types";

/** Product Loader default (`packages/xylitol-tui` LoaderIndicatorOptions). */
export const SPINNER_FRAMES = [
  "⠋",
  "⠙",
  "⠹",
  "⠸",
  "⠼",
  "⠴",
  "⠦",
  "⠧",
  "⠇",
  "⠏",
] as const;

export const DEFAULT_SPIN_MS = 80;

const SPIN_SET = new Set<string>(SPINNER_FRAMES);

function firstCodePoint(text: string): string {
  return Array.from(text)[0] ?? "";
}

function restCodePoints(text: string): string {
  return Array.from(text).slice(1).join("");
}

export function spinFrames(state: StateDoc): string[] {
  if (state.spin?.frames?.length) return state.spin.frames;
  if (hasSpin(state)) return [...SPINNER_FRAMES];
  return [];
}

export function spinMs(state: StateDoc): number {
  return state.spin?.ms ?? DEFAULT_SPIN_MS;
}

export function hasSpin(state: StateDoc): boolean {
  if (state.spin?.frames?.length) return true;
  for (const row of state.lines) {
    for (const span of row) {
      if (span.spin) return true;
      if (SPIN_SET.has(firstCodePoint(span.text))) return true;
    }
  }
  return false;
}

export function applyFrame(state: StateDoc, frame: number): StateDoc {
  const frames = spinFrames(state);
  if (!frames.length) return state;
  const glyph = frames[((frame % frames.length) + frames.length) % frames.length];
  return {
    ...state,
    lines: state.lines.map((row) => row.map((span) => paintSpinSpan(span, glyph))),
  };
}

function paintSpinSpan(span: Span, glyph: string): Span {
  if (span.spin) {
    const tail = SPIN_SET.has(firstCodePoint(span.text))
      ? restCodePoints(span.text)
      : span.text;
    return { ...span, text: `${glyph}${tail}` };
  }
  const head = firstCodePoint(span.text);
  if (!SPIN_SET.has(head)) return span;
  return { ...span, text: `${glyph}${restCodePoints(span.text)}` };
}
