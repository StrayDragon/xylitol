import type { StateDoc } from "./types";

export function flattenLines(state: StateDoc): string {
  return state.lines
    .map((row) => row.map((span) => span.text).join(""))
    .join("\n");
}

export function renderGrid(state: StateDoc): HTMLElement {
  const grid = document.createElement("div");
  grid.className = "cell-grid";
  grid.style.setProperty("--cols", String(state.cols ?? 80));
  for (const row of state.lines) {
    const line = document.createElement("div");
    line.className = "cell-row";
    for (const span of row) {
      const el = document.createElement("span");
      el.className = "cell";
      if (span.token) el.dataset.token = span.token;
      el.textContent = span.text;
      line.appendChild(el);
    }
    grid.appendChild(line);
  }
  return grid;
}
