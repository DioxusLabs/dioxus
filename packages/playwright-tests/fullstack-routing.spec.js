// @ts-check
const { test, expect } = require("@playwright/test");

// Wait for the build to finish
async function waitForBuild(request) {
  for (let i = 0; i < 10; i++) {
    const build = await request.get("http://localhost:8888");
    let text = await build.text();
    if (!text.includes("Backend connection failed")) {
      return;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
}

// The home and id routes should return 200
test("home route", async ({ request }) => {
  await waitForBuild(request);
  const response = await request.get("http://localhost:8888");

  expect(response.status()).toBe(200);

  const text = await response.text();
  expect(text).toContain("Home");
});

test("blog route", async ({ request }) => {
  await waitForBuild(request);
  const response = await request.get("http://localhost:8888/blog/123");

  expect(response.status()).toBe(200);

  const text = await response.text();
  expect(text).toContain("id: 123");
});

// The error route should return 500
test("error route", async ({ request }) => {
  await waitForBuild(request);
  const response = await request.get("http://localhost:8888/error");

  expect(response.status()).toBe(500);
});

// The async error route should return 500
test("async error route", async ({ request }) => {
  await waitForBuild(request);
  const response = await request.get("http://localhost:8888/async-error");

  expect(response.status()).toBe(500);

  // Expect the response to contain the error message
  const errorMessage = "Async error from a server function";
  const text = await response.text();
  expect(text).toContain(errorMessage);
});

// An unknown route should return 404
test("unknown route", async ({ request }) => {
  await waitForBuild(request);
  const response = await request.get(
    "http://localhost:8888/this-route-does-not-exist"
  );

  expect(response.status()).toBe(404);
});

// Clicking the link on the home page should navigate to the blog route
test("click blog link", async ({ page }) => {
  await page.goto("http://localhost:8888");

  // Click the link to the blog route
  await page.locator('a').click();

  // Wait for navigation to complete
  await page.waitForURL("http://localhost:8888/blog/1/");

  // Check that the blog page is displayed
  const text = await page.textContent("body");
  expect(text).toContain("id: 1");
});

// Clicking the link on the blog page should navigate back to the home route
test("click home link from blog", async ({ page }) => {
  await page.goto("http://localhost:8888/blog/1");

  // Click the link to the home route
  await page.locator('a').click();

  // Wait for navigation to complete
  await page.waitForURL("http://localhost:8888");

  // Check that the home page is displayed
  const text = await page.textContent("body");
  expect(text).toContain("Home");
});
