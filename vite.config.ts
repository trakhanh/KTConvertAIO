import { defineConfig } from "vite";
import { resolve } from "node:path";

const root = import.meta.dirname;

export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        wheel: resolve(root, "wheel.html"),
        toast: resolve(root, "toast.html"),
        settings: resolve(root, "settings.html"),
      },
    },
  },
});
