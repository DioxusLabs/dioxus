// @ts-check
const { test, expect } = require("@playwright/test");

test("errors", async ({ page }) => {
  await page.goto("http://localhost:3232");

  // give the page time to finish loading resources
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(2000);

  // Make sure the error that was thrown on the server is shown in the error boundary on the client
  const errors = page.locator("#error-fallback-button");
  await expect(errors).toContainText("Error fallback button clicked 0 times");
  // Make sure the fallback is interactive
  await errors.click();
  await expect(errors).toContainText("Error fallback button clicked 1 times");
});
