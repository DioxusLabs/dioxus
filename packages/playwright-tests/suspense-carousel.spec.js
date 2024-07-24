// @ts-check
const { test, expect } = require("@playwright/test");

test("suspense resolves on server", async ({ page }) => {
  // Wait for the dev server to reload
  await page.goto("http://localhost:4040");
  // Then wait for the page to start loading
  await page.goto("http://localhost:4040", { waitUntil: "commit" });

  // On the client, we should see some loading text
  const main = page.locator("#main");
  await expect(main).toContainText("Loading...");

  await page.waitForTimeout(1000);

  // Expect the page to contain the suspense result from the server
  await expect(main).toContainText("outer suspense result: Server");

  // And more loading text for the nested suspense
  await expect(main).toContainText("Loading... more");

  await page.waitForTimeout(1000);

  // And the nested suspense result
  await expect(main).toContainText("nested suspense result: Server");

  // Click the outer button
  let button = page.locator("button#outer-button-0");
  await button.click();
  // The button should have incremented
  await expect(button).toContainText("1");

  // Click the nested button
  button = page.locator("button#nested-button-0");
  await button.click();
  // The button should have incremented
  await expect(button).toContainText("1");

  // Now incrementing the carousel should create a new suspense boundary
  let incrementCarouselButton = page.locator(
    "button#increment-carousel-button"
  );
  await incrementCarouselButton.click();

  // A new pending suspense should be created on the client
  await expect(main).toContainText("Loading...");

  // The suspense should resolve on the client
  let newSuspense = page.locator("#outer-3");
  await expect(newSuspense).toContainText("outer suspense result: Client");

  // It should be loading more
  await expect(newSuspense).toContainText("Loading... more");

  // And the nested suspense result
  await expect(newSuspense).toContainText("nested suspense result: Client");

  // Click the outer button
  button = page.locator("button#outer-button-3");
  await button.click();
  // The button should have incremented
  await expect(button).toContainText("1");

  // Click the nested button
  button = page.locator("button#nested-button-3");
  await button.click();
  // The button should have incremented
  await expect(button).toContainText("1");
});
