// @ts-check
const { test, expect } = require("@playwright/test");

const hotPatchTimeout = {
  timeout: 1000 * 60 * 2, // 2 minute
};

test("button click", async ({ page }) => {
  const fs = require("fs");
  const mainPath = "web-hot-patch-fullstack-temp/src/main.rs";
  var mainContent = fs.readFileSync(mainPath, "utf8");
  const stylePath = "web-hot-patch-fullstack-temp/assets/style.css";
  var styleContent = fs.readFileSync(stylePath, "utf8");

  // Reset any changes made to the main.rs and style.css files.
  mainContent = mainContent.replace(/Ok\(\s*2\s*\)/g, "Ok(1)");
  mainContent = mainContent.replace("Click button! Count:", "Click me! Count:");
  mainContent = mainContent.replace("asset!('/assets/alternative-style.css')", "asset!('/assets/style.css')");
  fs.writeFileSync(mainPath, mainContent);
  styleContent = styleContent.replace(
    "background-color: blue;",
    "background-color: red;"
  );
  fs.writeFileSync(stylePath, styleContent);

  await page.goto("http://localhost:9981");


  // wait a sec for the serverfn to process
  // give the page time to finish loading resources
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(2000);

  // ** First test make sure the initial fat build is working **
  // Expect the page to contain the counter text.
  const main = page.locator("#main");
  await expect(main).toContainText("Click me! Count: 0");

  // Click the increment button.
  let button = page.locator("button#increment-button");
  await button.click();

  await page.waitForTimeout(1000);

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText("Click me! Count: 1");

  // Make sure the css is applied correctly.
  await expect(main).toHaveCSS("background-color", "rgb(255, 0, 0)");

  // Make sure the image is loaded.
  const headerImage = page.locator("#toasts");
  // expect the attribute src to start with /assets/toasts-
  await expect(headerImage).toHaveAttribute("src", /\/assets\/toasts-/);

  // ** Then make sure the hot patch is working **
  // Then change the file to increment by 2.
  const updatedContent = mainContent.replace(/Ok\(\s*1\s*\)/g, "Ok(2)");
  // Change the click me text to reflect the new increment.
  const updatedContentWithText = updatedContent.replace(
    "Click me! Count:",
    "Click button! Count:"
  );
  fs.writeFileSync(mainPath, updatedContentWithText);

  // Wait for the page to update and show the new text.
  await expect(main).toContainText("Click button! Count: 1", hotPatchTimeout);

  // Now click the button again.
  await button.click();

  // wait a sec for the serverfn to process
  await page.waitForTimeout(1000);

  // Expect the count to update by 2.
  await expect(main).toContainText("Click button! Count: 3");

  // Next change just the css file to change the background color to blue.
  const updatedStyleContent = styleContent.replace(
    "background-color: red;",
    "background-color: blue;"
  );
  fs.writeFileSync(stylePath, updatedStyleContent);

  // Wait for the page to update the background color.
  await expect(main).toHaveCSS(
    "background-color",
    "rgb(0, 0, 255)",
    hotPatchTimeout
  );

  // Make sure the image is still loaded.
  // expect the attribute src to start with /assets/toasts-
  await expect(headerImage).toHaveAttribute("src", /\/assets\/toasts-/);

  // ** Then add a new asset to the page **
  // Switch from style to alternative-style
  const updatedContentWithAlternativeStyle = updatedContentWithText.replace(
    'asset!("/assets/style.css")',
    'asset!("/assets/alternative-style.css")'
  );
  fs.writeFileSync(mainPath, updatedContentWithAlternativeStyle);

  // Assert the page has the new alternative style applied.
  const body = page.locator("body");
  // Log the page content to debug if needed.
  console.log(await page.content());
  await expect(body).toHaveCSS(
    "background-color",
    "rgb(100, 100, 100)",
    hotPatchTimeout
  );
});
