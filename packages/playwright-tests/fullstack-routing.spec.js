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

// An unknown route should return 404
test("unknown route", async ({ request }) => {
  await waitForBuild(request);
  const response = await request.get(
    "http://localhost:8888/this-route-does-not-exist"
  );

  expect(response.status()).toBe(404);
});
