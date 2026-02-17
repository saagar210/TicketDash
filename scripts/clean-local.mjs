#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const mode = process.argv[2] ?? "heavy";
const root = path.resolve(new URL("..", import.meta.url).pathname);

const heavyTargets = [
  "dist",
  "dist-ssr",
  "coverage",
  ".vite",
  ".turbo",
  ".cache",
  "node_modules/.vite",
  "src-tauri/target",
  "src-tauri/gen/schemas",
  ".DS_Store",
  "src/.DS_Store",
  "src-tauri/.DS_Store",
];

const fullOnlyTargets = [
  "node_modules",
  ".codex_audit",
  ".eslintcache",
];

const targets = mode === "full" ? [...heavyTargets, ...fullOnlyTargets] : heavyTargets;
let removed = 0;

for (const relPath of targets) {
  const targetPath = path.join(root, relPath);
  try {
    fs.rmSync(targetPath, { recursive: true, force: true, maxRetries: 8, retryDelay: 100 });
    removed += 1;
  } catch (error) {
    console.error(`Failed to remove ${relPath}:`, error);
    process.exitCode = 1;
  }
}

console.log(`Cleanup mode: ${mode}. Processed ${removed}/${targets.length} paths.`);
