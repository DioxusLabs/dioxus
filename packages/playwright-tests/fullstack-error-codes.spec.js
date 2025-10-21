// @ts-check
const { test, expect } = require("@playwright/test");

test("errors", async ({ page }) => {
  // Make sure going to home returns a 200
  const res = await page.goto("http://localhost:8124");
  if (!res) {
    throw new Error("Failed to navigate to http://localhost:8124");
  }

  expect(res.status()).toBe(200);

  // Go to /blog/1 which should also be okay
  const blogRes = await page.goto("http://localhost:8124/blog/1");
  if (!blogRes) {
    throw new Error("Failed to navigate to http://localhost:8124/blog/1");
  }
  expect(blogRes.status()).toBe(200);

  // Go to /blog/2 which should be a 201
  const blogErrorRes = await page.goto("http://localhost:8124/blog/2");
  if (!blogErrorRes) {
    throw new Error("Failed to navigate to http://localhost:8124/blog/2");
  }
  expect(blogErrorRes.status()).toBe(201);

  // Go to /blog/3 which should be a 500
  const blogPanicRes = await page.goto("http://localhost:8124/blog/3");
  if (!blogPanicRes) {
    throw new Error("Failed to navigate to http://localhost:8124/blog/3");
  }
  expect(blogPanicRes.status()).toBe(500);

  // Go to /blog/4 which should be a 404
  const blogNotFoundRes = await page.goto("http://localhost:8124/blog/4");
  if (!blogNotFoundRes) {
    throw new Error("Failed to navigate to http://localhost:8124/blog/4");
  }
  expect(blogNotFoundRes.status()).toBe(404);
});
