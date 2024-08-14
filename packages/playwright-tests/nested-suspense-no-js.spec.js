// @ts-check
const { test, expect } = require("@playwright/test");

test.use({ javaScriptEnabled: false });

test("text appears in the body without javascript", async ({ page }) => {
  await page.goto("http://localhost:5050", { waitUntil: "commit" });
  // Wait for the page to finish building. Reload until it's ready
  for (let i = 0; i < 50; i++) {
    // If the page doesn't contain #building or "Backend connection failed", we're ready
    let building_count = await page.locator("#building").count();
    building_count += await page
      .locator("body", { hasText: "backend connection failed" })
      .count();
    if (building_count === 0) {
      break;
    }
    await page.waitForTimeout(1000);
    await page.goto("http://localhost:5050", { waitUntil: "commit" });
  }
  // If we wait until the whole page loads, the content of the site should still be in the body even if javascript is disabled
  // It will not be visible, and may not be in the right order/location, but SEO should still work
  await page.waitForLoadState("load");

  const body = page.locator("body");
  const textExpected = [
    "The robot becomes sentient and says hello world",
    "The world says hello back",
    "In a stunning turn of events, the world collectively unites and says hello back",
    "Goodbye Robot",
    "The robot says goodbye",
    "Goodbye World",
    "The world says goodbye",
    "Hello World",
    "The world says hello again",
  ];
  for (let i = 0; i < textExpected.length; i++) {
    await expect(body).toContainText(textExpected[i]);
  }
});
