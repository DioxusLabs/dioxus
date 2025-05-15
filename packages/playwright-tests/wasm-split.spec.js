// @ts-check
const { test, expect } = require("@playwright/test");

test("wasm-split page is functional", async ({ page }) => {
  // Wait for the dev server to load
  await page.goto("http://localhost:8001");

  // Make sure the local button works - no broken wasm
  const counter = page.locator("#counter-display");
  await expect(counter).toContainText("Count: 1");
  await page.locator("#increment-counter").click();
  await expect(counter).toContainText("Count: 2");

  // Make sure the global button works - no broken wasm
  const counterGlobal = page.locator("#global-counter");
  await expect(counterGlobal).toContainText("Global Counter: 0");
  await page.locator("#increment-counter-global").click();
  await expect(counterGlobal).toContainText("Global Counter: 1");

  // Fire one of the wasm modules to load. Should update the counter and add some text
  const addBodyTextButton = page.locator("#add-body-text");
  await addBodyTextButton.click();
  await expect(counterGlobal).toContainText("Global Counter: 2");
  const outputBox = page.locator("#output-box");
  await expect(outputBox).toContainText("Rendered!");

  // The other wasm module
  const addBodyElementButton = page.locator("#add-body-element");
  await addBodyElementButton.click();
  await expect(counterGlobal).toContainText("Global Counter: 4");
  await expect(outputBox).toContainText("Some inner div");

  // Load the gzip and brotli modules
  const gzipButton = page.locator("#gzip-it");
  await gzipButton.click();
  await expect(counterGlobal).toContainText("Global Counter: 7");
  const brotliButton = page.locator("#brotli-it");
  await brotliButton.click();
  await expect(counterGlobal).toContainText("Global Counter: 11");

  // Ignore the requests in CI
  // Load the other router module
  const childRouteButton = page.locator("#link-child");
  await childRouteButton.click();
  const nestedChildCounter = page.locator("#nested-child-count");
  await expect(nestedChildCounter).toContainText("Count: hello");
  await page.locator("#nested-child-add-world").click();
  await expect(nestedChildCounter).toContainText("Count: hello world");
});
