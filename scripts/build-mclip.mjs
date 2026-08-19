import { spawnSync } from "node:child_process";
import { mkdirSync, copyFileSync } from "node:fs";
import { join } from "node:path";

const build = spawnSync("cargo", ["build", "--bin", "mclip", "--release", "--manifest-path", "src-tauri/Cargo.toml"], {
  stdio: "inherit",
  shell: process.platform === "win32",
});
if (build.error || build.status !== 0) process.exit(1);

const rustc = spawnSync("rustc", ["-Vv"], { encoding: "utf8" });
if (rustc.error || rustc.status !== 0) process.exit(1);

const hostLine = rustc.stdout
  .split("\n")
  .map((l) => l.trim())
  .find((l) => l.startsWith("host:"));
if (!hostLine) process.exit(1);

const triple = hostLine.slice(5).trim();

const isWindows = triple.includes("windows");
const ext = isWindows ? ".exe" : "";
const src = join("src-tauri", "target", "release", `mclip${ext}`);
const dest = join("src-tauri", "binaries", `mclip-${triple}${ext}`);

mkdirSync(join("src-tauri", "binaries"), { recursive: true });
copyFileSync(src, dest);

console.log(dest);
