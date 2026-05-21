// @ts-nocheck
import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const currentDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(currentDir, "..");
const host = process.env.TAURI_DEV_HOST;
const securityMarkdown = readFileSync(resolve(repoRoot, "SECURITY.md"), "utf8");

function gitOutput(command) {
  try {
    return execSync(command, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "ignore"],
    })
      .toString()
      .trim();
  } catch {
    return "";
  }
}

const buildCommit = process.env.VITE_GIT_COMMIT || gitOutput("git rev-parse --short=12 HEAD");
const buildTag = process.env.VITE_GIT_TAG || gitOutput("git tag --points-at HEAD").split(/\s+/)[0] || "";

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],
  define: {
    __WHISPRA_SECURITY_MD__: JSON.stringify(securityMarkdown),
    __WHISPRA_BUILD_COMMIT__: JSON.stringify(buildCommit),
    __WHISPRA_BUILD_TAG__: JSON.stringify(buildTag),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
