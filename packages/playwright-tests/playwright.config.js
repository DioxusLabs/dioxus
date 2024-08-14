// @ts-check
const { defineConfig, devices } = require("@playwright/test");
const path = require("path");

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// require('dotenv').config();

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
    trace: "on-first-retry",
    // Increase the timeout for navigations to give dx time to build the project
    navigationTimeout: 50 * 60 * 1000,
  },

  timeout: 50 * 60 * 1000,

  /* Configure projects for major browsers */
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },

    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },

    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },

    /* Test against mobile viewports. */
    // {
    //   name: 'Mobile Chrome',
    //   use: { ...devices['Pixel 5'] },
    // },
    // {
    //   name: 'Mobile Safari',
    //   use: { ...devices['iPhone 12'] },
    // },

    /* Test against branded browsers. */
    // {
    //   name: 'Microsoft Edge',
    //   use: { ...devices['Desktop Edge'], channel: 'msedge' },
    // },
    // {
    //   name: 'Google Chrome',
    //   use: { ..devices['Desktop Chrome'], channel: 'chrome' },
    // },
  ],

  /* Run your local dev server before starting the tests */
  webServer: [
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
        'cargo run --package dioxus-cli --release -- serve --addr "127.0.0.1" --port 9999',
      port: 9999,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "static-generation"),
      command:
        'cargo run --package dioxus-cli --release -- serve --force-sequential --platform static-generation --addr "127.0.0.1" --port 2222',
      port: 2222,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "fullstack"),
      command:
        'cargo run --package dioxus-cli --release -- serve --force-sequential --platform fullstack --addr "127.0.0.1" --port 3333',
      port: 3333,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "suspense-carousel"),
      command:
        'cargo run --package dioxus-cli --release -- serve --force-sequential --platform fullstack --addr "127.0.0.1" --port 4040',
      port: 4040,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
    {
      cwd: path.join(process.cwd(), "nested-suspense"),
      command:
        'cargo run --package dioxus-cli --release -- serve --force-sequential --platform fullstack --addr "127.0.0.1" --port 5050',
      port: 5050,
      timeout: 50 * 60 * 1000,
      reuseExistingServer: !process.env.CI,
      stdout: "pipe",
    },
  ],
});
