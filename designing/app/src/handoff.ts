import { formatUrl } from "./route";
import type { Route } from "./route";
import type { ModulePreview } from "./types";

export type HandoffInput = {
  route: Route;
  mod: ModulePreview;
  origin: string;
  lastAction: string;
};

export function handoffPaths(input: HandoffInput): string[] {
  const { route, mod } = input;
  return [
    `designing/${mod.surface}/modules/${mod.id}/intent.md`,
    `designing/${mod.surface}/modules/${mod.id}/draft.yaml`,
    `designing/${mod.surface}/modules/${mod.id}/states/${route.state}.yaml`,
    "designing/AGENTS.md",
    "designing/generated/AGENT-INDEX.md",
    "src/app/tui/DESIGN.md",
    "src/app/tui/",
  ];
}

function metaLines(input: HandoffInput): string[] {
  const { route, mod, origin, lastAction } = input;
  const endpoint = formatUrl(route);
  return [
    `endpoint: ${endpoint}`,
    `url: ${origin}${endpoint}`,
    `surface: ${mod.surface}`,
    `module: ${mod.id}`,
    `title: ${mod.title}`,
    `state: ${route.state}`,
    `scheme: ${route.scheme}`,
    `play: ${route.play ? 1 : 0}`,
    `frame: ${route.frame}`,
    `last_action: ${lastAction || "opened"}`,
  ];
}

export function handoffCopy(input: HandoffInput): string {
  return [...metaLines(input), "", ...handoffPaths(input)].join("\n");
}

export function handoffMarkdown(input: HandoffInput): string {
  return [
    ...metaLines(input).map((line) => {
      const i = line.indexOf(": ");
      const key = line.slice(0, i);
      const val = line.slice(i + 2);
      return `- **${key}:** \`${val}\``;
    }),
    "",
    ...handoffPaths(input).map((p) => `- \`${p}\``),
  ].join("\n");
}
