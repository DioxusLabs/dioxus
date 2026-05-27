// @ts-check
const { test: base, expect } = require("@playwright/test");
const fs = require("fs");

const test = base.extend({
  browser: async ({ playwright }, use) => {
    const browser = await playwright.chromium.connectOverCDP("http://127.0.0.1:8788");
    await use(browser);
    await browser.close();
  },
  context: async ({ browser }, use) => {
    const context = browser.contexts()[0];
    await use(context);
  },
  page: async ({ context }, use) => {
    const page = context.pages()[0];
    await use(page);
  },
});

const hotPatchTimeout = { timeout: 1000 * 60 * 2 };

test("windows fullstack hotpatch", async ({ page }) => {
  const mainPath = "windows-hotpatch-fullstack-temp/src/main.rs";
  const stylePath = "windows-hotpatch-fullstack-temp/assets/style.css";
  let mainContent = fs.readFileSync(mainPath, "utf8");
  let styleContent = fs.readFileSync(stylePath, "utf8");

  // Reset any changes from prior runs.
  mainContent = mainContent.replace(/Ok\(\s*2\s*\)/g, "Ok(1)");
  mainContent = mainContent.replace("Click button! Count:", "Click me! Count:");
  fs.writeFileSync(mainPath, mainContent);
  styleContent = styleContent.replace("background-color: blue;", "background-color: red;");
  fs.writeFileSync(stylePath, styleContent);

  // ** Verify the initial fat build is working **
  const main = page.locator("#main");
  await expect(main).toContainText("Click me! Count: 0");

  // Click and verify the server function works.
  const button = page.locator("button#increment-button");
  await button.click();
  await page.waitForTimeout(1000);
  await expect(main).toContainText("Click me! Count: 1");

  // Verify initial CSS is applied.
  await expect(main).toHaveCSS("background-color", "rgb(255, 0, 0)");

  // ** Hot patch: change server fn return value and button text **
  const updatedContent = mainContent
    .replace(/Ok\(\s*1\s*\)/g, "Ok(2)")
    .replace("Click me! Count:", "Click button! Count:");
  fs.writeFileSync(mainPath, updatedContent);

  // Wait for the hot patch to apply (text should update).
  await expect(main).toContainText("Click button! Count: 1", hotPatchTimeout);

  // Click and verify the new increment amount from the patched server fn.
  await button.click();
  await page.waitForTimeout(5000);
  await expect(main).toContainText("Click button! Count: 3");

  // ** Hot patch CSS **
  const updatedStyle = styleContent.replace(
    "background-color: red;",
    "background-color: blue;"
  );
  fs.writeFileSync(stylePath, updatedStyle);

  await page.waitForTimeout(1000);

  // Wait for the CSS hot patch to apply.
  await expect(main).toHaveCSS(
    "background-color",
    "rgb(0, 0, 255)",
    hotPatchTimeout
  );
});
