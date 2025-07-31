// @ts-check
const { test, expect } = require("@playwright/test");

test("external wasm-bindgen scripts are loaded", async ({ page }) => {
  await page.goto("http://localhost:9898");

  // Expect the page to load the script after optimizations have been applied. The script
  // should add a div to the page that says "Hello from Foo"
  const main = page.locator("#main");
  await expect(main).toContainText("Hello from Foo");
});
