const { expect, defineConfig } = require("@playwright/test");
import { test as base } from '@playwright/test';
import fs from 'fs';
import os from 'os';
import path from 'path';
import childProcess from 'child_process';


export const test = base.extend({
  browser: async ({ playwright }, use, testInfo) => {
    const browser = await playwright.chromium.connectOverCDP(`http://127.0.0.1:8787`);
    await use(browser);
    await browser.close();
  },
  context: async ({ browser }, use) => {
    const context = browser.contexts()[0];
    await use(context);
  },
  page: async ({ context }, use) => {
    const page = context.pages()[0];
    await use(page);
  },
});

test("button click", async ({ page }) => {
  // Expect the page to contain the counter text.
  const main = page.locator("#main");
  await expect(main).toContainText("High-five counter: 0");

  // Click the increment button.
  let button = page.locator("button#increment-button");
  await button.click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText("High-five counter: 1");
});

