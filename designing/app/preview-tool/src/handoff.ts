import { formatUrl } from "./route";
import type { Route } from "./route";
import type { ModulePreview } from "./types";

export type HandoffInput = {
  route: Route;
  mod: ModulePreview | null;
  origin: string;
  lastAction: string;
};

export function handoffPaths(input: HandoffInput): string[] {
  const { route, mod } = input;
  const base = "designing/user_interfaces";
  // shell 视图（state 为空）：路径指向所选区域的模块文档 + 帧与区域注解。
  if (!mod || !route.state) {
    const id = route.id || "";
    return [
      ...(id
        ? [
            `${base}/tui/modules/${id}/intent.md`,
            `${base}/tui/modules/${id}/draft.yaml`,
          ]
        : []),
      `${base}/tui/shell.regions.yaml`,
      "designing/app/preview-tool/generated/shell-frame.json",
      "designing/AGENTS.md",
      "designing/INDEX.md",
      "src/app/tui/DESIGN.md",
      "src/app/tui/",
    ];
  }
  return [
    `${base}/${mod.surface}/modules/${mod.id}/intent.md`,
    `${base}/${mod.surface}/modules/${mod.id}/draft.yaml`,
    `${base}/${mod.surface}/modules/${mod.id}/states/${route.state}.yaml`,
    "designing/AGENTS.md",
    "designing/INDEX.md",
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
    `surface: ${mod?.surface ?? route.surface}`,
    `module: ${route.id || mod?.id || "—"}`,
    ...(mod ? [`title: ${mod.title}`] : []),
    `state: ${route.state || "shell"}`,
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
