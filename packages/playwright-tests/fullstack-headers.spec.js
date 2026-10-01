// @ts-check
const { test, expect } = require("@playwright/test");

test("Set-Cookie headers are appended per RFC 6265", async ({ page }) => {
  await page.goto("http://localhost:3636", { waitUntil: "networkidle" });

  await page.click("#set-cookie-btn");

  const responsePromise = page.waitForResponse((resp) => {
    return resp.url().includes("/api/test_set_cookie") && resp.status() === 200;
  });

  const response = await responsePromise;
  const headers = await response.headersArray();
  const setCookies = headers.filter(
    (h) => h.name.toLowerCase() === "set-cookie",
  );
  expect(setCookies.length).toBe(2);
  const setCookieValues = setCookies.map((h) => h.value);
  expect(setCookieValues).toEqual([
    "session_id=abc123; Path=/",
    "theme=dark; Path=/",
  ]);
});

test("non-Set-Cookie headers are overwritten", async ({ page }) => {
  await page.goto("http://localhost:3636", { waitUntil: "networkidle" });

  await page.click("#override-header-btn");

  const responsePromise = page.waitForResponse((resp) => {
    return (
      resp.url().includes("/api/test_override_header") && resp.status() === 200
    );
  });

  const response = await responsePromise;
  const headers = await response.headersArray();
  const xCustomHeader = headers.filter(
    (h) => h.name.toLowerCase() === "x-custom-header",
  );
  expect(xCustomHeader.length).toBe(1);
  const xCustomHeaderValues = xCustomHeader.map((h) => h.value);
  expect(xCustomHeaderValues[0]).toBe("second");
});
