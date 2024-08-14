// @ts-check
const { test, expect } = require("@playwright/test");
const { timeout } = require("./playwright.config");

test("nested suspense resolves", async ({ page }) => {
  // Wait for the dev server to reload
  await page.goto("http://localhost:5050");
  // Then wait for the page to start loading
  await page.goto("http://localhost:5050", { waitUntil: "commit" });

  // On the client, we should see some loading text
  const main = page.locator("#main");
  await expect(main).toContainText("Loading 0...");

  // Expect the page to contain the suspense result from the server
  const mainMessageDiv = page.locator("#children-0");
  const mainMessageTitle = page.locator("#title-0");
  await expect(mainMessageTitle).toContainText("The robot says hello world");
  const mainMessageBody = page.locator("#body-0");
  await expect(mainMessageBody).toContainText(
    "The robot becomes sentient and says hello world"
  );

  // And expect the title to have resolved on the client
  await expect(page).toHaveTitle("The robot says hello world");

  // And more loading text for the nested suspense
  await expect(mainMessageDiv).toContainText("Loading 1...");
  await expect(mainMessageDiv).toContainText("Loading 2...");
  await expect(mainMessageDiv).toContainText("Loading 3...");

  const nestedMessageDiv1 = page.locator("#children-1");
  const nestedMessageTitle1 = page.locator("#title-1");
  await expect(nestedMessageTitle1).toContainText("The world says hello back");
  const nestedMessageBody1 = page.locator("#body-1");
  await expect(nestedMessageBody1).toContainText(
    "In a stunning turn of events, the world collectively unites and says hello back"
  );

  const nestedMessageDiv2 = page.locator("#children-2");
  await expect(nestedMessageDiv2).toBeEmpty();
  const nestedMessageTitle2 = page.locator("#title-2");
  await expect(nestedMessageTitle2).toContainText("Goodbye Robot");
  const nestedMessageBody2 = page.locator("#body-2");
  await expect(nestedMessageBody2).toContainText("The robot says goodbye");

  const nestedMessageDiv3 = page.locator("#children-3");
  await expect(nestedMessageDiv3).toBeEmpty();
  const nestedMessageTitle3 = page.locator("#title-3");
  await expect(nestedMessageTitle3).toContainText("Goodbye World");
  const nestedMessageBody3 = page.locator("#body-3");
  await expect(nestedMessageBody3).toContainText("The world says goodbye");

  // Even more loading text for the deeply nested suspense
  await expect(nestedMessageDiv1).toContainText("Loading 4...");

  const nestedMessageDiv4 = page.locator("#children-4");
  await expect(nestedMessageDiv4).toBeEmpty();
  const nestedMessageTitle4 = page.locator("#title-4");
  await expect(nestedMessageTitle4).toContainText("Hello World");
  const nestedMessageBody4 = page.locator("#body-4");
  await expect(nestedMessageBody4).toContainText("The world says hello again");
});
