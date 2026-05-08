// @ts-check
const { defineConfig, devices } = require("@playwright/test");
const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

// Copy a directory to a temp location for tests that modify source files (hot-patch tests).
// Done in JS so the webServer command is a simple `dx serve` with a `cwd`, which keeps
// stdout/stderr piping clean on both Unix and Windows cmd.exe.
function copyToTemp(src, dest) {
  const absSrc = path.resolve(__dirname, src);
  const absDest = path.resolve(__dirname, dest);
  fs.rmSync(absDest, { recursive: true, force: true });
  fs.cpSync(absSrc, absDest, { recursive: true });
}

const repoRoot = path.resolve(__dirname, "..", "..");
const dx = path.join(repoRoot, "target", "release", process.platform === "win32" ? "dx.exe" : "dx");

const ALL_SERVERS = [
  {
    specs: ["liveview.spec.js"], port: 3030,
    command: "cargo run --package dioxus-playwright-liveview-test --bin dioxus-playwright-liveview-test",
    env: { CARGO_TERM_PROGRESS_WHEN: "never" }
  },
  { specs: ["web.spec.js"], port: 9990, cwd: "web", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 9990` },
  { specs: ["web-routing.spec.js"], port: 2020, cwd: "web-routing", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 2020` },
  { specs: ["web-hash-routing.spec.js"], port: 2021, cwd: "web-hash-routing", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 2021` },
  { specs: ["fullstack.spec.js"], port: 3333, cwd: "fullstack", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 3333 --release` },
  { specs: ["fullstack-errors.spec.js"], port: 3232, cwd: "fullstack-errors", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 3232` },
  { specs: ["fullstack-mounted.spec.js"], port: 7777, cwd: "fullstack-mounted", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 7777` },
  { specs: ["fullstack-routing.spec.js"], port: 8888, cwd: "fullstack-routing", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 8888` },
  { specs: ["fullstack-spread.spec.js"], port: 7980, cwd: "fullstack-spread", command: `${dx} run --verbose --force-sequential --web --addr 127.0.0.1 --port 7980` },
  { specs: ["fullstack-hydration-order.spec.js"], port: 7979, cwd: "fullstack-hydration-order", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 7979` },
  { specs: ["fullstack-error-codes.spec.js"], port: 8124, cwd: "fullstack-error-codes", command: `${dx} run --force-sequential --addr 127.0.0.1 --port 8124` },
  { specs: ["nested-suspense.spec.js", "nested-suspense-no-js.spec.js"], port: 5050, cwd: "nested-suspense", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 5050` },
  { specs: ["nested-suspense-ssg.spec.js"], port: 6060, cwd: "nested-suspense", command: `${dx} run --bin nested-suspense-ssg --force-sequential --web --ssg --addr 127.0.0.1 --port 6060` },
  { specs: ["suspense-carousel.spec.js"], port: 4040, cwd: "suspense-carousel", command: `${dx} run --force-sequential --web --addr 127.0.0.1 --port 4040` },
  { specs: ["cli-optimization.spec.js"], port: 8989, cwd: "cli-optimization", command: `${dx} run --addr 127.0.0.1 --port 8989` },
  { specs: ["wasm-split.spec.js"], port: 8001, cwd: "wasm-split-harness", command: `${dx} run --bin wasm-split-harness --web --addr 127.0.0.1 --port 8001 --wasm-split --profile wasm-split-release` },
  { specs: ["default-features-disabled.spec.js"], port: 8002, cwd: "default-features-disabled", command: `${dx} run --force-sequential --addr 127.0.0.1 --port 8002` },
  { specs: ["web-patch.spec.js"], port: 9980, cwd: "web-hot-patch-temp", setup: () => copyToTemp("web-hot-patch", "web-hot-patch-temp"), command: `${dx} serve --verbose --force-sequential --web --addr 127.0.0.1 --port 9980 --hot-patch --exit-on-error` },
  { specs: ["web-patch-fullstack.spec.js"], port: 9981, cwd: "web-hot-patch-fullstack-temp", setup: () => copyToTemp("web-hot-patch-fullstack", "web-hot-patch-fullstack-temp"), command: `${dx} serve --verbose --force-sequential --web --addr 127.0.0.1 --port 9981 --hot-patch --exit-on-error` },
];

if (process.platform === "win32") {
  ALL_SERVERS.push({ specs: ["windows.spec.js"], port: 8787, cwd: "windows-headless", command: `${dx} run --force-sequential` });
  ALL_SERVERS.push({ specs: ["windows-hotpatch-fullstack.spec.js"], port: 8788, cwd: "windows-hotpatch-fullstack-temp", setup: () => copyToTemp("windows-hotpatch-fullstack", "windows-hotpatch-fullstack-temp"), command: `${dx} serve --verbose --force-sequential --hot-patch --exit-on-error` });
}

// Determine which servers to start based on spec files in argv and platform.
// On Windows only windows.spec.js runs (grep filter below), so don't bother
// starting servers for tests that will never execute — they just crash and
// pollute the logs.
const specArgs = process.argv.filter((a) => a.endsWith(".spec.js")).map((a) => path.basename(a));
const isWindows = process.platform === "win32";
const activeServers = specArgs.length > 0
  ? ALL_SERVERS.filter((s) => s.specs.some((spec) => specArgs.includes(spec)))
  : isWindows
    ? ALL_SERVERS.filter((s) => s.specs.some((spec) => spec.includes("windows")))
    : ALL_SERVERS.filter((s) => !s.specs.some((spec) => spec.includes("windows")));

// Run any setup functions (e.g. copying source to temp dirs for hot-patch tests)
// Only happens on initialize
//
// Build dx once: main process builds and sets env var; workers inherit it and skip.
// todo: implement proper fixtures https://www.youtube.com/watch?v=3i6cJUFO_m4
if (!process.env._DX_BUILT) {
  for (const s of activeServers) {
    if (s.setup) s.setup();
  }

  execSync("cargo build --package dioxus-cli --release", {
    cwd: repoRoot,
    env: { ...process.env, CARGO_TERM_PROGRESS_WHEN: "never" },
    stdio: "inherit",
  });

  process.env._DX_BUILT = "1";
}

module.exports = defineConfig({
  testDir: ".",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    trace: "retain-on-failure",
    navigationTimeout: 50 * 60 * 1000,
  },
  timeout: 50 * 60 * 1000,
  webServer: activeServers.map((s) => ({
    command: s.command,
    port: s.port,
    cwd: s.cwd ? path.join(__dirname, s.cwd) : __dirname,
    timeout: 50 * 60 * 1000,
    reuseExistingServer: !process.env.CI,
    stdout: "pipe",
    stderr: "pipe",
    env: s.env,
  })),
  projects: [
    {
      name: "chromium",
      grep: process.platform === "win32" ? /windows/ : undefined,
      grepInvert: process.platform !== "win32" ? /windows/ : undefined,
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
