// @ts-check
const { test, expect } = require("@playwright/test");

test("loads with correct features", async ({ page }) => {
  await page.goto("http://localhost:8002");

  // Expect the page to contain the pending text.
  const main = page.locator("#main");
  await expect(main).toContainText('server features: ["server"]');
  await expect(main).toContainText('client features: ["web"]');
});
