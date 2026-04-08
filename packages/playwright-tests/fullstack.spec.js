// @ts-check
const { test, expect } = require("@playwright/test");

test("hydration", async ({ page }) => {
  await page.goto("http://localhost:3333");

  // Then wait for the page to start loading
  await page.goto("http://localhost:3333", { waitUntil: "networkidle" });

  // Expect the page to contain the pending text.
  const main = page.locator("#main");
  await expect(main).toContainText("Server said: ...");

  // Expect the page to contain the counter text.
  await expect(main).toContainText("hello axum! 12345");
  // Expect the title to contain the counter text.
  await expect(page).toHaveTitle("hello axum! 12345");

  // Click the increment button.
  let button = page.locator("button.increment-button");
  await button.click();

  // Click the server button.
  let serverButton = page.locator("button.server-button");
  await serverButton.click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText("hello axum! 12346");
  // Expect the title to contain the updated counter text.
  await expect(page).toHaveTitle("hello axum! 12346");

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText("Server said: Hello from the server!");

  // Make sure the error that was thrown on the server is shown in the error boundary on the client
  const errors = page.locator("#errors");
  await expect(errors).toContainText("Hmm, something went wrong.");

  // Expect the onmounted event to be called exactly once.
  const mountedDiv = page.locator("div.onmounted-div");
  await expect(mountedDiv).toHaveText("onmounted was called 1 times");
});

test("document elements", async ({ page }) => {
  await page.goto("http://localhost:3333");
  // wait until the meta element is mounted
  const meta = page.locator("meta#meta-head[name='testing']");
  await meta.waitFor({ state: "attached" });
  await expect(meta).toHaveAttribute("data", "dioxus-meta-element");

  const link = page.locator("link#link-head[rel='stylesheet']");
  await link.waitFor({ state: "attached" });
  await expect(link).toHaveAttribute(
    "href",
    "https://fonts.googleapis.com/css?family=Roboto+Mono"
  );

  const stylesheet = page.locator("link#stylesheet-head[rel='stylesheet']");
  await stylesheet.waitFor({ state: "attached" });
  await expect(stylesheet).toHaveAttribute(
    "href",
    "https://fonts.googleapis.com/css?family=Roboto:300,300italic,700,700italic"
  );

  const script = page.locator("script#script-head");
  await script.waitFor({ state: "attached" });
  await expect(script).toHaveAttribute("async", "true");

  const style = page.locator("style#style-head");
  await style.waitFor({ state: "attached" });
  const main = page.locator("#main");
  await expect(main).toHaveCSS("font-family", "Roboto");
});

test("assets cache correctly", async ({ page }) => {
  // Wait for the hashed image to be loaded
  const hashedImageFuture = page.waitForResponse((resp) => {
    console.log("Response URL:", resp.url());
    return resp.url().includes("/assets/image-") && resp.status() === 200;
  });
  const assetImageFuture = page.waitForResponse(
    (resp) => resp.url().includes("/assets/image.png") && resp.status() === 200
  );
  const nestedAssetImageFuture = page.waitForResponse(
    (resp) =>
      resp.url().includes("/assets/nested/image.png") && resp.status() === 200
  );

  // Navigate to the page that includes the image.
  await page.goto("http://localhost:3333");

  const hashedImageResponse = await hashedImageFuture;

  // Make sure the hashed image cache control header is set to immutable
  const cacheControl = hashedImageResponse.headers()["cache-control"];
  console.log("Cache-Control header:", cacheControl);
  expect(cacheControl).toContain("immutable");

  // Wait for the asset image to be loaded
  const assetImageResponse = await assetImageFuture;
  // console.log("Asset Image Response:", assetImageResponse);
  // Make sure the asset image cache control header does not contain immutable
  const assetCacheControl = assetImageResponse.headers()["cache-control"];
  console.log("Cache-Control header:", assetCacheControl);
  // Expect there to be no cache control header
  expect(assetCacheControl).toBeFalsy();

  // Wait for the nested asset image to be loaded
  const nestedAssetImageResponse = await nestedAssetImageFuture;
  // console.log(
  //   "Nested Asset Image Response:",
  //   nestedAssetImageResponse
  // );
  // Make sure the nested asset image cache control header does not contain immutable
  const nestedAssetCacheControl =
    nestedAssetImageResponse.headers()["cache-control"];
  console.log("Cache-Control header:", nestedAssetCacheControl);
  // Expect there to be no cache control header
  expect(nestedAssetCacheControl).toBeFalsy();
});

test("websockets", async ({ page }) => {
  await page.goto("http://localhost:3333");
  // wait until the websocket div is mounted
  const wsDiv = page.locator("div#websocket-div");
  await expect(wsDiv).toHaveText("Received: HELLO WORLD");
});

