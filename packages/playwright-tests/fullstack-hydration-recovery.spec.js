// @ts-check
const { test, expect } = require("@playwright/test");

const SERVER_URL = "http://localhost:7978";
const HYDRATION_MISMATCH_MESSAGE = "[HYDRATION MISMATCH]";
const HYDRATION_RECOVERY_MESSAGE =
  "Rebuilding subtree.";

async function waitForBuild(request) {
  for (let i = 0; i < 30; i++) {
    const response = await request.get(SERVER_URL);
    const text = await response.text();
    if (response.status() === 200 && text.includes('id="recovery-button"')) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw new Error("Timed out waiting for the hydration recovery fixture to build");
}

test("hydration mismatch recovers nested structure, text, attributes, and placeholders", async ({
  page,
  request,
}) => {
  await waitForBuild(request);

  const serverResponse = await request.get(SERVER_URL);
  expect(serverResponse.status()).toBe(200);

  const serverHtml = await serverResponse.text();
  expect(serverHtml).toContain('id="recovery-button"');
  expect(serverHtml).toContain('id="after-streaming-boundary"');
  expect(serverHtml).toMatch(/<div\b[^>]*id="recovery-button"/);
  expect(serverHtml).not.toMatch(/<button\b[^>]*id="recovery-button"/);
  expect(serverHtml).toContain("Server text content");
  expect(serverHtml).toContain("Server placeholder content");
  expect(serverHtml).toContain('title="Server value title"');
  expect(serverHtml).toContain('data-side="server"');
  expect(serverHtml).toContain("Shared inner html");
  expect(serverHtml).toContain("Server dangerous inner html");
  expect(serverHtml).toContain('style="width:100px;height:40px;"');
  expect(serverHtml).toContain('id="server-extra-node"');
  expect(serverHtml).not.toContain('role="status"');
  expect(serverHtml).not.toContain('title="Client attribute title"');

  const consoleMessages = [];
  const consoleErrors = [];
  const pageErrors = [];

  page.on("console", (msg) => {
    consoleMessages.push(msg.text());
    if (msg.type() === "error") {
      consoleErrors.push(msg.text());
    }
  });
  page.on("pageerror", (error) => {
    pageErrors.push(error.message);
  });

  await page.goto(SERVER_URL, { waitUntil: "domcontentloaded" });
  await expect(page.locator("#streaming-fallback")).toHaveText("Loading streaming…");
  await page.waitForLoadState("networkidle");

  const mismatchMessages = () =>
    consoleMessages.filter((message) =>
      message.includes(HYDRATION_MISMATCH_MESSAGE),
    );
  const hasMismatch = (...fragments) =>
    mismatchMessages().some((message) =>
      fragments.every((fragment) => message.includes(fragment)),
    );

  await expect
    .poll(() => mismatchMessages().length, {
      message: "expected hydration mismatches to be logged in debug builds",
    })
    .toBeGreaterThan(0);
  await expect
    .poll(
      () => consoleMessages.some((message) => message.includes(HYDRATION_RECOVERY_MESSAGE)),
      { message: "expected hydration recovery to be logged" },
    )
    .toBeTruthy();

  expect(
    mismatchMessages().every(
      (message) =>
        message.includes("Reason:") &&
        message.includes("--- expected") &&
        message.includes("+++ actual") &&
        message.includes("@@"),
    ),
  ).toBeTruthy();

  expect(
    hasMismatch(
      "Reason: Expected <button>, found <div>.",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Reason: Expected <strong>, found <span>.",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      'Reason: Expected text "Client text content", found text "Server text content".',
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "the DOM is missing [role, title]",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Reason: Expected placeholder (comment node), found node type 1.",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "these values differ [data-side: expected \"client\", found \"server\", title: expected \"Client value title\", found \"Server value title\"]",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      'Reason: Expected text "  Client whitespace content  ", found text "Client whitespace content".',
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Expected no additional child nodes",
      "Server extra node",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Reason: Expected <button>, found <div>.",
      "Streaming mismatch",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "these values differ [style:",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Expected dangerous_inner_html content to match",
      "Client dangerous inner html",
    ),
  ).toBeTruthy();
  expect(
    mismatchMessages().every(
      (message) => !message.includes("dangerous-inner-html-stable-shell"),
    ),
  ).toBeTruthy();

  const recoveryButton = page.locator("#recovery-button");
  await expect(recoveryButton).toHaveCount(1);
  await expect(recoveryButton).toHaveJSProperty("tagName", "BUTTON");
  await expect(recoveryButton).toHaveText("Recovered 0");

  const nestedLeaf = page.locator("#nested-leaf");
  await expect(nestedLeaf).toHaveCount(1);
  await expect(nestedLeaf).toHaveJSProperty("tagName", "STRONG");
  await expect(nestedLeaf).toHaveText("Nested client leaf");

  const textMismatch = page.locator("#text-mismatch");
  await expect(textMismatch).toHaveText("Client text content");

  const attributeMismatch = page.locator("#attribute-mismatch");
  await expect(attributeMismatch).toHaveAttribute("role", "status");
  await expect(attributeMismatch).toHaveAttribute(
    "title",
    "Client attribute title",
  );

  const attributeValueMismatch = page.locator("#attribute-value-mismatch");
  await expect(attributeValueMismatch).toHaveAttribute(
    "title",
    "Client value title",
  );
  await expect(attributeValueMismatch).toHaveAttribute("data-side", "client");

  const dangerousInnerHtml = page.locator("#dangerous-inner-html-stable");
  await expect(dangerousInnerHtml.locator("#dangerous-inner-html-child")).toHaveText(
    "Shared inner html",
  );

  const dangerousInnerHtmlMismatch = page.locator("#dangerous-inner-html-mismatch");
  await expect(
    dangerousInnerHtmlMismatch.locator("#dangerous-inner-html-mismatch-child"),
  ).toHaveText("Client dangerous inner html");
  await expect(
    dangerousInnerHtmlMismatch.locator("#dangerous-inner-html-mismatch-child"),
  ).toHaveJSProperty("tagName", "STRONG");

  const styleMismatch = page.locator("#style-mismatch");
  await expect(styleMismatch).toHaveCSS("width", "200px");
  await expect(styleMismatch).toHaveCSS("height", "50px");

  const whitespaceMismatch = page.locator("#whitespace-mismatch");
  await expect.poll(() => whitespaceMismatch.evaluate((node) => node.textContent)).toBe(
    "  Client whitespace content  ",
  );

  await expect(page.locator("#server-extra-node")).toHaveCount(0);
  await expect(page.locator("#extra-node-stable")).toHaveText("Shared child");

  await expect(page.locator("#placeholder-mismatch-shell p")).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText("Server text content");
  await expect(page.locator("body")).not.toContainText(
    "Server placeholder content",
  );
  await expect(page.locator("body")).not.toContainText("Server extra node");
  await expect(page.locator("body")).not.toContainText("Server dangerous inner html");

  await recoveryButton.click();
  await expect(recoveryButton).toHaveText("Recovered 1");
  await recoveryButton.click();
  await expect(recoveryButton).toHaveText("Recovered 2");

  // Streaming mismatch recovery: the suspense boundary resolved from the
  // server, the client detected a tag mismatch, and rebuilt the subtree.
  const streamingMismatch = page.locator("#streaming-mismatch");
  await expect(streamingMismatch).toHaveCount(1);
  await expect(streamingMismatch).toHaveJSProperty("tagName", "BUTTON");
  await expect(streamingMismatch).toContainText("Streaming client: streamed data");

  await expect
    .poll(() =>
      page.evaluate(() => {
        const appRoot = document.querySelector("#app-root");
        const streamingShell = document.querySelector("#streaming-mismatch-shell");
        const afterBoundary = document.querySelector("#after-streaming-boundary");
        if (!appRoot || !streamingShell || !afterBoundary) {
          return "missing";
        }

        const children = Array.from(appRoot.children);
        return (
          children.indexOf(streamingShell) < children.indexOf(afterBoundary)
        );
      }),
    )
    .toBe(true);

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
