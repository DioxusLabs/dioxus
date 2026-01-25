// @ts-check
const { defineConfig, devices } = require("@playwright/test");
const path = require("path");

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// require('dotenv').config();

let webServer = [];
let grep = undefined;
let grepInvert = undefined;
if (process.platform === "win32") {
  webServer = [
    {
      cwd: path.join(process.cwd(), "windows-headless"),
      command:
        "cargo run --package dioxus-cli --release -- run --force-sequential",
      port: 8787,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
  ];
  grep = /windows/;
} else {
  webServer = [
    {
      command:
        "cargo run --package dioxus-playwright-liveview-test --bin dioxus-playwright-liveview-test",
      port: 3030,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "web"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 9990',
      port: 9990,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "web-routing"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 2020',
      port: 2020,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "web-hash-routing"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 2021',
      port: 2021,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 3333 --release',
      port: 3333,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack-errors"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 3232',
      port: 3232,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack-mounted"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 7777',
      port: 7777,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack-spread"),
      command:
        'cargo run --package dioxus-cli --release -- run --verbose --force-sequential --web --addr "127.0.0.1" --port 7980',
      port: 7980,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack-routing"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 8888',
      port: 8888,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "suspense-carousel"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 4040',
      port: 4040,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "nested-suspense"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 5050',
      port: 5050,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "nested-suspense"),
      command:
        'cargo run --package dioxus-cli --release -- run --bin nested-suspense-ssg --force-sequential --web --ssg --addr "127.0.0.1" --port 6060',
      port: 6060,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack-hydration-order"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --web --addr "127.0.0.1" --port 7979',
      port: 7979,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "cli-optimization"),
      command:
        'cargo run --package dioxus-cli --release -- run --addr "127.0.0.1" --port 8989',
      port: 8989,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "wasm-split-harness"),
      command:
        'cargo run --package dioxus-cli --release -- run --bin wasm-split-harness --web --addr "127.0.0.1" --port 8001 --wasm-split --profile wasm-split-release',
      port: 8001,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "default-features-disabled"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --addr "127.0.0.1" --port 8002',
      port: 8002,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "barebones-template"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --addr "127.0.0.1" --port 8123',
      port: 8123,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      command:
        'rm -rf ./web-hot-patch-temp && cp -r ./web-hot-patch ./web-hot-patch-temp && cd web-hot-patch-temp && cargo run --manifest-path ../../cli/Cargo.toml --release -- serve --verbose --force-sequential --web --addr "127.0.0.1" --port 9980 --hot-patch --exit-on-error',
      port: 9980,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      command:
        'rm -rf ./web-hot-patch-fullstack-temp && cp -r ./web-hot-patch-fullstack ./web-hot-patch-fullstack-temp && cd web-hot-patch-fullstack-temp && cargo run --manifest-path ../../cli/Cargo.toml --release -- serve --verbose --force-sequential --web --addr "127.0.0.1" --port 9981 --hot-patch --exit-on-error',
      port: 9981,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack-error-codes"),
      command:
        'cargo run --package dioxus-cli --release -- run --force-sequential --addr "127.0.0.1" --port 8124',
      port: 8124,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
  ];

  grepInvert = /windows/;
}

/**
 * @see https://playwright.dev/docs/test-configuration
 */
module.exports = defineConfig({
  testDir: ".",
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: "html",
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    // baseURL: 'http://127.0.0.1:3000',

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: "retain-on-failure",
    // Increase the timeout for navigations to give dx time to build the project
    navigationTimeout: 50 * 60 * 1000,
  },

  timeout: 50 * 60 * 1000,

  /* Configure projects for major browsers */
  projects: [
    {
      name: "chromium",
      grep,
      grepInvert,
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  /* Run your local dev server before starting the tests */
  webServer,
});
