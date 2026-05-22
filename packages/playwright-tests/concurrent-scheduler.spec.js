// @ts-check
const { test, expect } = require("@playwright/test");

const PORT = 9099;
const DOT_COUNT = 2187;
const MAX_ALLOWED_GAP_MS = 140;

test.use({ trace: "off" });

test("fiber triangle yields while ticking every dot", async ({ page }) => {
  test.setTimeout(120000);
  page.setDefaultTimeout(10000);

  await page.goto(`http://127.0.0.1:${PORT}`);

  await expect(page.getByText("Fiber Triangle")).toBeVisible();
  await expect(page.locator("#dot-count")).toHaveText(String(DOT_COUNT));
  await expect(page.locator(".dot")).toHaveCount(DOT_COUNT, { timeout: 45000 });

  await page.waitForFunction(
    () => Number(document.querySelector("#second-count")?.textContent ?? "0") >= 2,
    undefined,
    { timeout: 15000 }
  );

  await page.waitForFunction(
    () => Number(document.querySelector("#fiber-yields")?.textContent ?? "0") > 0,
    undefined,
    { timeout: 15000 }
  );

  await page.waitForTimeout(500);

  const metrics = await page.evaluate(() => {
    const text = (id) => document.querySelector(id)?.textContent ?? "0";
    return {
      worstGap: Number(text("#worst-gap").replace("ms", "")),
      jankFrames: Number(text("#jank-frames")),
      fiberWork: Number(text("#fiber-work")),
      fiberCommits: Number(text("#fiber-commits")),
      fiberYields: Number(text("#fiber-yields")),
      second: Number(text("#second-count")),
    };
  });

  expect(metrics.second).toBeGreaterThanOrEqual(2);
  expect(metrics.fiberWork).toBeGreaterThan(DOT_COUNT);
  expect(metrics.fiberCommits).toBeGreaterThan(0);
  expect(metrics.fiberYields).toBeGreaterThan(0);
  expect(metrics.worstGap).toBeLessThan(MAX_ALLOWED_GAP_MS);
});
