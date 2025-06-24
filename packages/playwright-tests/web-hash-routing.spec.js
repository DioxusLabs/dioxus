// @ts-check
const { test, expect } = require("@playwright/test");

test("redirect", async ({ page }) => {
  // Going to the root url should redirect to /other.
  await page.goto("http://localhost:2021");

  // Expect the page to the text Other
  const main = page.locator("#other");
  await expect(main).toContainText("Other");

  // Expect the url to be /#/other
  await expect(page).toHaveURL("http://localhost:2021/#/other");
});

test("links", async ({ page }) => {
  await page.goto("http://localhost:2021/#/other");

  // Expect clicking the link to /other/123 to navigate to /other/123
  const link = page.locator("a[href='/#/other/123']");
  await link.click();
  await expect(page).toHaveURL("http://localhost:2021/#/other/123");
});

test("fallback", async ({ page }) => {
  await page.goto("http://localhost:2021/#/my/404/route");

  // Expect the page to contain the text Fallback
  const main = page.locator("#not-found");
  await expect(main).toContainText("NotFound");
});
