// @ts-check
const { test, expect } = require("@playwright/test");

test("hydration", async ({ page }) => {
  await page.goto("http://localhost:7777");

  // Expect the page to contain the pending text.
  const main = page.locator("#main");
  await expect(main).toContainText("The mounted event was triggered.");
});
