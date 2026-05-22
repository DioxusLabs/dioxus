// @ts-check
const { test, expect } = require("@playwright/test");

test("hydration", async ({ page }) => {
  await page.goto("http://localhost:7979");

  // give time for the page to load and hydrate
  await page.waitForTimeout(2000);

  // Expect the page to contain a button
  const button = page.locator("#counter");
  await expect(button).toContainText("Count 0");

  // Hydration should succeed and clicking the button should increase the count
  await button.click();
  await expect(button).toContainText("Count 1");
});

// Regression test: in markerless hydration, multiple trailing empty dynamic
// texts after a non-empty one must be inserted in document source order. The
// reverse-order bug only surfaces once the empties become non-empty.
test("trailing empty dynamic texts hydrate in source order", async ({ page }) => {
  await page.goto("http://localhost:7979");
  await page.waitForTimeout(2000);

  const div = page.locator("#trailing-empties");
  // Before fill: only the leading non-empty contributes visible text.
  await expect(div).toHaveText("FIRST");

  await page.locator("#fill-trailing").click();
  // After fill: must read "FIRST[a][b]", not "FIRST[b][a]".
  await expect(div).toHaveText("FIRST[a][b]");
});
