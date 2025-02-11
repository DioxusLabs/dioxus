// @ts-check
const { test, expect } = require("@playwright/test");

test("optimized scripts run", async ({ page }) => {
  await page.goto("http://localhost:8989");

  // Expect the page to load the script after optimizations have been applied. The script
  // should add an editor to the page that shows a main function
  const main = page.locator("#main");
  await expect(main).toContainText("hi");
});
