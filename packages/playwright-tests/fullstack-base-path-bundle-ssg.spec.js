// @ts-check
const { test, expect } = require("@playwright/test");

test("should return 404 at the root URL", async ({ page }) => {
  const response = await page.goto('http://localhost:8080');
  expect(response).not.toBeNull();
  expect(response.status()).toBe(404);
});

test("should display 'Hello World!' content at /base-path", async ({ page }) => {
  await page.goto("http://localhost:8080/base-path");
  const main = page.locator("#main");
  await expect(main).toContainText("Hello World!");
});
