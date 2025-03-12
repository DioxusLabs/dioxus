// @ts-check
const { test, expect } = require("@playwright/test");

// The home and id routes should return 200
test("home route", async ({ request }) => {
  const response = await request.get("/");

  expect(response.status()).toBe(200);

  const text = await response.text();
  expect(text).toContain("Home");
});

test("blog route", async ({ request }) => {
  const response = await request.get("/blog/123");

  expect(response.status()).toBe(200);

  const text = await response.text();
  expect(text).toContain("id: 123");
});

// The error route should return 500
test("error route", async ({ request }) => {
  const response = await request.get("/error");

  expect(response.status()).toBe(500);
});

// An unknown route should return 404
test("unknown route", async ({ request }) => {
  const response = await request.get("/this-route-does-not-exist");

  expect(response.status()).toBe(404);
});
