// @ts-check
const { test, expect } = require("@playwright/test");

const scrolled = 1000;

function scrollY(page) {
  return page.evaluate(() => window.scrollY);
}

function nextFrame(page) {
  return page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => resolve(null)))
  );
}

async function openScrolled(page, base) {
  await page.goto(`${base}/scroll?page=1`);
  await expect(page.locator("#scroll")).toContainText("Scroll 1");
  await page.evaluate((y) => window.scrollTo(0, y), scrolled);
  await expect.poll(() => scrollY(page)).toBe(scrolled);
}

function press(page, id) {
  return page.locator(`#${id}`).dispatchEvent("click");
}

exports.scrollOnReplaceTests = (base) => {
  test("replace that only changes the query keeps the scroll position", async ({ page }) => {
    await openScrolled(page, base);
    await press(page, "replace-query");
    await expect(page.locator("#scroll")).toContainText("Scroll 2");
    await expect(page).toHaveURL(/\/scroll\?page=2/);
    await nextFrame(page);
    expect(await scrollY(page)).toBe(scrolled);
  });

  test("replace that only changes the fragment keeps the scroll position", async ({ page }) => {
    await openScrolled(page, base);
    await press(page, "replace-fragment");
    await expect(page).toHaveURL(/\/scroll\?page=1#section-target$/);
    await nextFrame(page);
    expect(await scrollY(page)).toBe(scrolled);
  });

  test("replace to another path scrolls to the top", async ({ page }) => {
    await openScrolled(page, base);
    await press(page, "replace-path");
    await expect(page.locator("#elsewhere")).toContainText("Elsewhere");
    await expect(page).toHaveURL(/\/elsewhere$/);
    await expect.poll(() => scrollY(page)).toBe(0);
  });

  test("push scrolls to the top and going back restores the scroll position", async ({ page }) => {
    await openScrolled(page, base);
    await press(page, "push-path");
    await expect(page.locator("#elsewhere")).toContainText("Elsewhere");
    await expect(page).toHaveURL(/\/elsewhere$/);
    await expect.poll(() => scrollY(page)).toBe(0);
    await page.goBack();
    await expect(page.locator("#scroll")).toContainText("Scroll 1");
    await expect.poll(() => scrollY(page)).toBe(scrolled);
  });
};
