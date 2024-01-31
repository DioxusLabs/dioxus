// @ts-check
const { test, expect, defineConfig } = require('@playwright/test');

test('button click', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the counter text.
  const main = page.locator('#main');
  await expect(main).toContainText('hello axum! 0');

  // Click the increment button.
  let button = page.locator('button.increment-button');
  await button.click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText('hello axum! 1');
});

test('svg', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the svg.
  const svg = page.locator('svg');

  // Expect the svg to contain the circle.
  const circle = svg.locator('circle');
  await expect(circle).toHaveAttribute('cx', '50');
  await expect(circle).toHaveAttribute('cy', '50');
  await expect(circle).toHaveAttribute('r', '40');
  await expect(circle).toHaveAttribute('stroke', 'green');
  await expect(circle).toHaveAttribute('fill', 'yellow');
});

test('raw attribute', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the div with the raw attribute.
  const div = page.locator('div.raw-attribute-div');
  await expect(div).toHaveAttribute('raw-attribute', 'raw-attribute-value');
});

test('hidden attribute', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the div with the hidden attribute.
  const div = page.locator('div.hidden-attribute-div');
  await expect(div).toHaveAttribute('hidden', 'true');
});

test('dangerous inner html', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the div with the dangerous inner html.
  const div = page.locator('div.dangerous-inner-html-div');
  await expect(div).toContainText('hello dangerous inner html');
});

test('input value', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the input with the value.
  const input = page.locator('input');
  await expect(input).toHaveValue('hello input');
});

test('style', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the div with the style.
  const div = page.locator('div.style-div');
  await expect(div).toHaveText('colored text');
  await expect(div).toHaveCSS('color', 'rgb(255, 0, 0)');
});

test('eval', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Expect the page to contain the div with the eval and have no text.
  const div = page.locator('div.eval-result');
  await expect(div).toHaveText('');

  // Click the button to run the eval.
  let button = page.locator('button.eval-button');
  await button.click();

  // Check that the title changed.
  await expect(page).toHaveTitle('Hello from Dioxus Eval!');

  // Check that the div has the eval value.
  await expect(div).toHaveText('returned eval value');

});

// Shutdown the li