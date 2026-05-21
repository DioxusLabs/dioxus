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

  // Regression coverage for https://github.com/DioxusLabs/dioxus/issues/5512.
  // Both files are vanilla IIFE scripts injected via `with_static_head(true)`.
  // The 0.7.4–0.7.6 esbuild path wrapped them in a wrapper IIFE / `--format=esm`,
  // which dropped the global side-effect and produced `export` syntax under a
  // classic `<script>` tag. The asserted globals only get set if the file is
  // delivered as a classic script with its body intact.
  test(`classic js asset with_minify(false) is copied byte-for-byte in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__iife_classic_value);
    expect(value).toBe("ok-classic");
  });

  test(`classic js asset with_minify(true) stays a classic script in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__iife_minify_value);
    expect(value).toBe("ok-minify");
    const classicScript = page.locator('script[src*="iife_minify"]');
    await expect(classicScript).toHaveCount(1);
    await expect(classicScript).not.toHaveAttribute("type", "module");
  });

  test(`umd js asset is minified without being converted to a module in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__umd_minify_value);
    expect(value).toBe("ok-umd");
    const classicScript = page.locator('script[src*="umd_minify"]');
    await expect(classicScript).toHaveCount(1);
    await expect(classicScript).not.toHaveAttribute("type", "module");
  });

  test(`cjs js asset is treated as a classic script in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__cjs_classic_value);
    expect(value).toBe("ok-cjs");
    const classicScript = page.locator('script[src*="cjs_classic"]');
    await expect(classicScript).toHaveCount(1);
    await expect(classicScript).not.toHaveAttribute("type", "module");
  });

  test(`sweetalert2 UMD bundle from issue 5512 is copied and exported globally in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => ({
      // @ts-ignore
      hasGlobal: typeof window.Sweetalert2 === "function",
      // @ts-ignore
      hasFire: typeof window.Sweetalert2?.fire === "function",
    }));
    expect(value).toEqual({ hasGlobal: true, hasFire: true });
    const classicScript = page.locator('script[src*="sweetalert2.all.min"]');
    await expect(classicScript).toHaveCount(1);
    await expect(classicScript).not.toHaveAttribute("type", "module");
  });

  // `with_module(true)` should emit `<script type="module">` and preserve
  // module syntax during minification. The fixture uses `import.meta.url`,
  // which only parses when the script tag has `type="module"`.
  test(`js asset with_module(true) is loaded as an ES module in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__esm_module_value);
    expect(value).toBe("ok-module");
    const moduleScript = page.locator('script[type="module"][src*="esm_module"]');
    await expect(moduleScript).toHaveCount(1);
  });

  // Auto-detection: the CLI's has_module_syntax scan should recognise top-level
  // `export` and emit `<script type="module">` even without `with_module(true)`.
  test(`js asset with top-level export is auto-detected as ES module in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__esm_auto_value);
    expect(value).toBe("ok-auto");
    // import.meta only parses inside a real module; if the script were emitted
    // as classic the file would have errored at parse time.
    // @ts-ignore
    const metaType = await page.evaluate(() => window.__esm_auto_meta);
    expect(metaType).toBe("string");
    const moduleScript = page.locator('script[type="module"][src*="esm_auto"]');
    await expect(moduleScript).toHaveCount(1);
  });

  test(`js asset with relative ESM import is bundled as an ES module in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__esm_import_value);
    expect(value).toBe("ok-esm-import");
    const moduleScript = page.locator('script[type="module"][src*="esm_import_entry"]');
    await expect(moduleScript).toHaveCount(1);
  });

  test(`mjs extension is loaded as an ES module in ${name}`, async ({ page }) => {
    await page.goto(`http://localhost:${port}`);
    // @ts-ignore
    const value = await page.evaluate(() => window.__mjs_auto_value);
    expect(value).toBe("ok-mjs");
    const moduleScript = page.locator('script[type="module"][src*="mjs_auto"]');
    await expect(moduleScript).toHaveCount(1);
  });

}
