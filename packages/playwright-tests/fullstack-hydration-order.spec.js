// @ts-check
const { test, expect } = require("@playwright/test");

test("hydration", async ({ page }) => {
  await page.goto("http://localhost:7979");

  // Expect the page to contain a button
  const button = page.locator("#counter");
  await expect(button).toContainText("Count 0");

  // Hydration should succeed and clicking the button should increase the count
  await button.click();
  await expect(button).toContainText("Count 1");
});
