const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: __dirname,
  globalSetup: require.resolve("./setup.cjs"),
  testMatch: "cleanup.spec.cjs",
  workers: 1,
  outputDir: "../../../target/interpreter-cleanup-results",
  reporter: "list",
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
