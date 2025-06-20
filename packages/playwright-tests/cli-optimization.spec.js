// @ts-check
const { test, expect } = require("@playwright/test");

test("optimized scripts run", async ({ page }) => {
  await page.goto("http://localhost:8989");

  // Expect the page to load the script after optimizations have been applied. The script
  // should add an editor to the page that shows a main function
  const main = page.locator("#main");
  await expect(main).toContainText("hi");

  // Expect the page to contain an image with the id "some_image"
  const image = page.locator("#some_image");
  await expect(image).toBeVisible();
  // Get the image src
  const src = await image.getAttribute("src");

  // Expect the page to contain an image with the id "some_image_with_the_same_url"
  const image2 = page.locator("#some_image_with_the_same_url");
  await expect(image2).toBeVisible();
  // Get the image src
  const src2 = await image2.getAttribute("src");

  // Expect the urls to be different
  expect(src).not.toEqual(src2);
});
