import { defineConfig, type Plugin } from "vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));

function surfaceSpaFallback(): Plugin {
  const rewrite = (url: string): string => {
    const [path, query] = url.split("?");
    if (
      path === "/tui" ||
      path === "/web" ||
      path.startsWith("/tui/") ||
      path.startsWith("/web/")
    ) {
      return query ? `/index.html?${query}` : "/index.html";
    }
    return url;
  };
  return {
    name: "surface-spa-fallback",
    configureServer(server) {
      server.middlewares.use((req, _res, next) => {
        if (req.url) req.url = rewrite(req.url);
        next();
      });
    },
    configurePreviewServer(server) {
      server.middlewares.use((req, _res, next) => {
        if (req.url) req.url = rewrite(req.url);
        next();
      });
    },
  };
}

export default defineConfig({
  root,
  publicDir: false,
  appType: "spa",
  plugins: [surfaceSpaFallback()],
  server: {
    port: 5173,
    open: "/tui/",
  },
  preview: {
    port: 5173,
  },
  resolve: {
    alias: {
      "@generated": resolve(root, "../generated"),
      "@modules": resolve(root, "../tui/modules"),
    },
  },
});
