import { defineConfig } from "vite";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root,
  publicDir: false,
  server: {
    port: 5173,
    open: true,
  },
  resolve: {
    alias: {
      "@generated": resolve(root, "../generated"),
      "@modules": resolve(root, "../modules"),
    },
  },
});
