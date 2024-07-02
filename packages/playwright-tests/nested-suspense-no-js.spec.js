// @ts-check
const { test, expect } = require("@playwright/test");

test.use({ javaScriptEnabled: false });

test("text appears in the body without javascript", async ({ page }) => {
  // If we wait until the whole page loads, the content of the site should still be in the body even if javascript is disabled
  // It will not be visible, and may not be in the right order/location, but SEO should still work
  await page.goto("http://localhost:5050");

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
