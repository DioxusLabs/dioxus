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

test("shared lazy routes load owned modules and shared dependencies", async ({ page, request }) => {
  await page.goto("http://localhost:8001");

  await page.locator("#link-shared-a").click();
  await expect(page.locator("#shared-a-marker")).toContainText("Shared route A: ab-a-shared-a");

  await page.locator("#link-shared-b").click();
  await expect(page.locator("#shared-b-marker")).toContainText("Shared route B: ab-b-shared-b");

  const wasmResources = await page.evaluate(() =>
    performance
      .getEntriesByType("resource")
      .map((entry) => entry.name)
      .filter((name) => name.includes(".wasm")),
  );
  const chunkResources = wasmResources.filter((name) => name.includes("chunk_"));
  const moduleResources = wasmResources.filter((name) => name.includes("module_"));
  expect(chunkResources, "shared Function/DataSymbol chunk was not requested").not.toHaveLength(0);
  expect(moduleResources.some((name) => name.includes("SharedRouteA")), "SharedRouteA module was not requested").toBeTruthy();
  expect(moduleResources.some((name) => name.includes("SharedRouteB")), "SharedRouteB module was not requested").toBeTruthy();

  for (const chunkUrl of chunkResources) {
    const response = await request.get(chunkUrl);
    expect(response.ok(), `shared chunk request failed: ${chunkUrl}`).toBeTruthy();
    const body = await response.body();
    expect(body.subarray(0, 4).toString("hex"), `not a wasm shared chunk: ${chunkUrl}`).toBe("0061736d");
  }

  const loaderScripts = await page.evaluate(() =>
    performance
      .getEntriesByType("resource")
      .map((entry) => entry.name)
      .filter((name) => name.endsWith(".js")),
  );
  const loaderText = (await Promise.all(loaderScripts.map(async (url) => (await request.get(url)).text()))).join("\n");
  const routeLoaderDependencies = (routeName) => {
    const route = loaderText.match(
      new RegExp(`module_\\d+_route${routeName}[^\\"]+\\.wasm\\",\\[([^\\]]*)\\]`),
    );
    expect(route, `${routeName} loader declaration was not found`).not.toBeNull();
    return route[1].split(",").map((dependency) => dependency.trim()).filter(Boolean);
  };
  const routeADependencies = routeLoaderDependencies("SharedRouteA");
  const routeBDependencies = routeLoaderDependencies("SharedRouteB");
  expect(routeADependencies.length, "SharedRouteA has no shared chunk dependency").toBeGreaterThan(0);
  expect(routeBDependencies.length, "SharedRouteB has no shared chunk dependency").toBeGreaterThan(0);
  expect(routeADependencies.some((dependency) => routeBDependencies.includes(dependency))).toBeTruthy();
});

test("four shared routes form distinct shared communities", async ({ page, request }) => {
  await page.goto("http://localhost:8001");
  for (const route of ["a", "b", "c", "d"]) {
    await page.locator(`#link-shared-${route}`).click();
    await expect(page.locator(`#shared-${route}-marker`)).toContainText(`Shared route ${route.toUpperCase()}`);
  }

  const scripts = await page.evaluate(() =>
    performance.getEntriesByType("resource").map((entry) => entry.name).filter((name) => name.endsWith(".js")),
  );
  const loaderText = (await Promise.all(scripts.map(async (url) => (await request.get(url)).text()))).join("\n");
  const chunks = [...new Set(loaderText.match(/__wasm_split_load_chunk_\d+/g) || [])];
  expect(chunks.length, "expected multiple shared chunks").toBeGreaterThanOrEqual(2);
});
