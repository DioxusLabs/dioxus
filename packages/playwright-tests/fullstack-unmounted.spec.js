// @ts-check
const { test, expect } = require("@playwright/test");

test("onunmounted event fires when element is removed", async ({ page }) => {
  await page.goto("http://localhost:7778");

  // First, verify the element is mounted and visible
  const testElement = page.locator("#test-element");
  await expect(testElement).toBeVisible();
  await expect(testElement).toContainText("Lifecycle test element");

  // Verify mounted event was triggered
  const mountedStatus = page.locator("#mounted-status");
  await expect(mountedStatus).toContainText("Element was mounted.");

  // Verify unmounted has NOT been triggered yet
  const unmountedStatus = page.locator("#unmounted-status");
  await expect(unmountedStatus).not.toBeVisible();

  // Click the toggle button to remove the element
  const toggleButton = page.locator("#toggle-button");
  await toggleButton.click();

  // Verify the element is no longer visible
  await expect(testElement).not.toBeVisible();

  // Verify the unmounted event was triggered
  await expect(unmountedStatus).toBeVisible();
  await expect(unmountedStatus).toContainText("The unmounted event was triggered.");
});

test("onunmounted event fires correctly on multiple toggle cycles", async ({ page }) => {
  await page.goto("http://localhost:7778");

  const testElement = page.locator("#test-element");
  const toggleButton = page.locator("#toggle-button");
  const mountedStatus = page.locator("#mounted-status");
  const unmountedStatus = page.locator("#unmounted-status");

  // Wait for hydration to complete by checking mounted event was triggered
  await expect(mountedStatus).toContainText("Element was mounted.");

  // Initial state: element visible, unmounted not triggered
  await expect(testElement).toBeVisible();
  await expect(unmountedStatus).not.toBeVisible();

  // First toggle: remove element
  await toggleButton.click();
  await expect(testElement).not.toBeVisible();
  await expect(unmountedStatus).toBeVisible();

  // Second toggle: add element back (unmounted status stays true from first removal)
  await toggleButton.click();
  await expect(testElement).toBeVisible();
  // The unmounted status should still show the previous unmount was triggered
  await expect(unmountedStatus).toContainText("The unmounted event was triggered.");
});
