// @ts-check
const { test, expect } = require("@playwright/test");

const URL = "http://localhost:7982";

test("scope with no SSR roots hydrates virtual root anchors", async ({
  page,
}) => {
  const res = await page.request.get(URL);
  const html = await res.text();
  const stripped = html.replace(
    /<script data-dioxus-hydration>[\s\S]*?<\/script>/g,
    ""
  );

  expect(stripped).not.toContain("late-empty-root");
  expect(stripped).not.toContain("root text ready");

  await page.goto(URL);
  await expect(page.locator("#late-empty-root")).toHaveText("late root ready");

  const bodyText = await page.evaluate(() => document.body.textContent || "");
  expect(bodyText).toContain("root text ready");
});
