// @ts-check
const { test, expect } = require("@playwright/test");

const test_variants = [
  { port: 8989, name: "current version" },
];

for (let { port, name } of test_variants) {
  test(`optimized scripts run in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);

    // // Expect the page to load the script after optimizations have been applied. The script
    // // should add an editor to the page that shows a main function
    // const main = page.locator("#main");
    // await expect(main).toContainText("hi");

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

    // Expect the page to contain an image with the id "some_image_without_hash"
    const image3 = page.locator("#some_image_without_hash");
    await expect(image3).toBeVisible();
    // Get the image src
    const src3 = await image3.getAttribute("src");
    // Expect the src to be without a hash
    expect(src3).toEqual("/assets/toasts.avif");
  });

  test(`unused external assets are bundled in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);

    // Assert http://localhost:{port}/assets/toasts.png is found even though it is not used in the page
    const response = await page.request.get(
      `http://localhost:${port}/assets/toasts.png`
    );
    // Expect the response to be ok
    expect(response.status()).toBe(200);
    // make sure the response is an image
    expect(response.headers()["content-type"]).toBe("image/png");
  });

  test(`assets are resolved in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);

    // Expect the page to contain an element with the id "resolved-data"
    const resolvedData = page.locator("#resolved-data");
    await expect(resolvedData).toBeVisible();
    // Expect the element to contain the text "List: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]"
    await expect(resolvedData).toContainText(
      "List: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]"
    );
  });

}
