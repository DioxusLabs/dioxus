// @ts-check
const { test, expect } = require("@playwright/test");

test("spread attributes hydrate", async ({ page }) => {
  await page.goto("http://localhost:7979");

  // Expect the page to contain the button
  const counter = page.locator("#counter");
  await expect(counter).toHaveText("Count: 0");
  // Clicking on the button should increment the count
  await counter.click();
  await expect(counter).toHaveText("Count: 1");
});
