import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { execSync } from "node:child_process";

const commit = execSync("git rev-parse --short HEAD", { encoding: "utf8" }).trim();
const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const icons = config.bundle?.icon ?? [];

console.log(`build commit: ${commit}`);
console.log("configured bundle icons:");

if (icons.length === 0) {
  console.error("No bundle icons configured in src-tauri/tauri.conf.json");
  process.exit(1);
}

for (const icon of icons) {
  const path = join("src-tauri", icon);
  const exists = existsSync(path);
  console.log(`- ${path}: ${exists ? "ok" : "missing"}`);

  if (!exists) {
    process.exitCode = 1;
  }
}

