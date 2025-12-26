// @ts-check
const { test, expect } = require("@playwright/test");

test("hydration", async ({ page }) => {
  await page.goto("http://localhost:7777");

  // Expect the page to contain the pending text.
  const mountedTest = page.locator("#mounted-test");
  await expect(mountedTest).toContainText("The mounted event was triggered.");
});

test("cleanup closure runs when element is removed", async ({ page }) => {
  await page.goto("http://localhost:7777");

  // Wait for hydration to complete - the mounted event should have fired
  const mountedTest = page.locator("#mounted-test");
  await expect(mountedTest).toContainText("The mounted event was triggered.");

  // Element with cleanup should be visible initially
  const cleanupElement = page.locator("#cleanup-test-element");
  await expect(cleanupElement).toBeVisible();

  // Cleanup indicator should not be visible yet
  const cleanupTriggered = page.locator("#cleanup-triggered");
  await expect(cleanupTriggered).not.toBeVisible();

  // Click button to remove the element
  const toggleButton = page.locator("#toggle-cleanup-element");
  await toggleButton.click();

  // Element should be removed
  await expect(cleanupElement).not.toBeVisible();

  // Cleanup should have been called
  await expect(cleanupTriggered).toBeVisible();
  await expect(cleanupTriggered).toContainText("Cleanup was called.");
});
