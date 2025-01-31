// @ts-check
const { test, expect } = require("@playwright/test");

test("hydration", async ({ page }) => {
  await page.goto("http://localhost:3333");

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
  await page.goto("http://localhost:9999");
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
