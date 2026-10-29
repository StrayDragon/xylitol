#!/usr/bin/env bun
type ToolSpec = {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
};

const TOOLS: ToolSpec[] = [
  {
    name: "echo",
    description: "Echo text back to the caller.",
    input_schema: {
      type: "object",
      properties: {
        text: { type: "string", description: "Text to echo." },
      },
      required: ["text"],
    },
  },
  {
    name: "add",
    description: "Add two integers.",
    input_schema: {
      type: "object",
      properties: {
        a: { type: "integer" },
        b: { type: "integer" },
      },
      required: ["a", "b"],
    },
  },
];

type Request = {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: Record<string, unknown>;
};

function reply(id: number, result: unknown) {
  console.log(JSON.stringify({ jsonrpc: "2.0", id, result }));
}

function fail(id: number, code: number, message: string) {
  console.log(JSON.stringify({ jsonrpc: "2.0", id, error: { code, message } }));
}

async function main() {
  const decoder = new TextDecoder();
  let buffer = "";
  for await (const chunk of Bun.stdin.stream()) {
    buffer += decoder.decode(chunk, { stream: true });
    let newline = buffer.indexOf("\n");
    while (newline >= 0) {
      const line = buffer.slice(0, newline).trim();
      buffer = buffer.slice(newline + 1);
      newline = buffer.indexOf("\n");
      if (!line) continue;
      let request: Request;
      try {
        request = JSON.parse(line);
      } catch {
        continue;
      }
      if (request.method === "initialize") {
        reply(request.id, {
          protocol_version: 1,
          name: "bun-demo",
          version: "0.1.0",
          capabilities: ["tools"],
          tools: TOOLS,
        });
        continue;
      }
      if (request.method === "tools/call") {
        const params = request.params ?? {};
        const tool = String(params.name ?? "");
        const args = (params.arguments ?? {}) as Record<string, unknown>;
        if (tool === "echo") {
          reply(request.id, { content: String(args.text ?? "") });
        } else if (tool === "add") {
          const a = Number(args.a ?? 0);
          const b = Number(args.b ?? 0);
          reply(request.id, { content: String(a + b) });
        } else {
          fail(request.id, -32601, `unknown tool: ${tool}`);
        }
        continue;
      }
      fail(request.id, -32601, `unknown method: ${request.method}`);
    }
  }
}

await main();
