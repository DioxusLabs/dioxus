// @ts-check
const { test, expect } = require("@playwright/test");

test("suspense resolves on server", async ({ page }) => {
  // Observe transient fallbacks before navigation. A browser can finish streaming
  // the nested result before Playwright resumes, so polling the current DOM alone
  // can miss a fallback that was correctly rendered.
  await page.addInitScript(() => {
    const state = {
      serverPending: false,
      serverNestedPending: false,
      clientPending: false,
      clientNestedPending: false,
      reversePending: false,
      reverseNestedPending: false,
      phase: "server",
    };
    /** @type {any} */ (window).__carouselSuspense = state;
    new MutationObserver(() => {
      const main = document.querySelector("#main");
      if (!main) return;
      const pending = /Loading\.\.\.(?! more)/.test(main.textContent ?? "");
      if (state.phase === "server") {
        state.serverPending ||= pending;
        state.serverNestedPending ||= document.querySelector("#outer-0")
          ?.textContent?.includes("Loading... more") ?? false;
      } else if (state.phase === "forward") {
        state.clientPending ||= pending;
        state.clientNestedPending ||= document.querySelector("#outer-3")
          ?.textContent?.includes("Loading... more") ?? false;
      } else {
        state.reversePending ||= pending;
        state.reverseNestedPending ||= document.querySelector("#outer-0")
          ?.textContent?.includes("Loading... more") ?? false;
      }
    }).observe(document, { childList: true, subtree: true, characterData: true });
  });
  await page.goto("http://localhost:4040", { waitUntil: "commit" });
  const main = page.locator("#main");
  await expect(main).toContainText("nested suspense result: Server");
  await expect.poll(() => page.evaluate(() =>
    /** @type {any} */ (window).__carouselSuspense.serverPending
  )).toBe(true);
  await expect(main).toContainText("outer suspense result: Server");
  await expect.poll(() => page.evaluate(() =>
    /** @type {any} */ (window).__carouselSuspense.serverNestedPending
  )).toBe(true);

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
  await page.evaluate(() => {
    /** @type {any} */ (window).__carouselSuspense.phase = "forward";
  });
  await incrementCarouselButton.click();

  // A new pending suspense should be created on the client
  await expect.poll(() => page.evaluate(() =>
    /** @type {any} */ (window).__carouselSuspense.clientPending
  )).toBe(true);

  // The suspense should resolve on the client
  let newSuspense = page.locator("#outer-3");
  await expect(newSuspense).toContainText("nested suspense result: Client");
  await expect(newSuspense).toContainText("outer suspense result: Client");

  // The nested fallback was rendered before the client result resolved.
  await expect.poll(() => page.evaluate(() =>
    /** @type {any} */ (window).__carouselSuspense.clientNestedPending
  )).toBe(true);

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

  // Now decrementing the carousel should create a new suspense boundary at the front
  let decrementCarouselButton = page.locator(
    "button#decrement-carousel-button"
  );
  await page.evaluate(() => {
    /** @type {any} */ (window).__carouselSuspense.phase = "reverse";
  });
  await decrementCarouselButton.click();

  // A new pending suspense should be created on the client
  await expect.poll(() => page.evaluate(() =>
    /** @type {any} */ (window).__carouselSuspense.reversePending
  )).toBe(true);

  // The suspense should resolve on the client
  newSuspense = page.locator("#outer-0");
  await expect(newSuspense).toContainText("nested suspense result: Client");
  await expect(newSuspense).toContainText("outer suspense result: Client");

  // The nested fallback was rendered before the client result resolved.
  await expect.poll(() => page.evaluate(() =>
    /** @type {any} */ (window).__carouselSuspense.reverseNestedPending
  )).toBe(true);

  // Click the outer button
  button = page.locator("button#outer-button-0");
  await button.click();
  // The button should have incremented
  await expect(button).toContainText("1");

  // Click the nested button
  button = page.locator("button#nested-button-0");
  await button.click();
  // The button should have incremented
  await expect(button).toContainText("1");
});
