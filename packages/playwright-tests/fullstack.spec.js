// @ts-check
const { test, expect } = require('@playwright/test');

test('button click', async ({ page }) => {
  await page.goto('http://localhost:3333');
  await page.waitForTimeout(1000);

  // Expect the page to contain the counter text.
  const main = page.locator('#main');
  await expect(main).toContainText('hello axum! 12345');

  // Click the increment button.
  let button = page.locator('button.increment-button');
  await button.click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText('hello axum! 12346');
});

test('fullstack communication', async ({ page }) => {
  await page.goto('http://localhost:3333');
  await page.waitForTimeout(1000);

  // Expect the page to contain the counter text.
  const main = page.locator('#main');
  await expect(main).toContainText('Server said: ...');

  // Click the increment button.
  let button = page.locator('button.server-button');
  await button.click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText('Server said: Hello from the server!');
});
