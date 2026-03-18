// @ts-check
const { test, expect } = require("@playwright/test");

const SERVER_URL = "http://localhost:7978";
const HYDRATION_MISMATCH_MESSAGE = "[HYDRATION MISMATCH]";
const HYDRATION_RECOVERY_MESSAGE =
  "Hydration mismatches detected. Falling back to a full client rebuild.";

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
  expect(serverHtml).toContain("Server text content");
  expect(serverHtml).toContain("Server placeholder content");
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

  await page.goto(SERVER_URL);
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
      message: "expected one warning for each mismatch class",
    })
    .toBe(4);
  await expect
    .poll(
      () =>
        consoleMessages.filter((message) =>
          message.includes(HYDRATION_RECOVERY_MESSAGE),
        ).length,
      { message: "expected the hydration fallback warning to be logged once" },
    )
    .toBe(1);

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
      "Reason: Expected <strong>, found <span>.",
      "-strong {",
      "+span {",
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      'Reason: Expected text "Client text content", found text "Server text content".',
      '-    "Client text content",',
      '+    "Server text content",',
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Reason: Expected <div> with attributes [role, title], but the DOM node is missing them.",
      'role: "status"',
      'title: "Client attribute title"',
    ),
  ).toBeTruthy();
  expect(
    hasMismatch(
      "Reason: Expected placeholder (comment node), found node type 1.",
      "VNode::placeholder()",
      "+    p {",
    ),
  ).toBeTruthy();

  const recoveryButton = page.locator("#recovery-button");
  await expect(recoveryButton).toHaveCount(1);
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

  await expect(page.locator("#placeholder-mismatch-shell p")).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText("Server text content");
  await expect(page.locator("body")).not.toContainText(
    "Server placeholder content",
  );

  await recoveryButton.click();
  await expect(recoveryButton).toHaveText("Recovered 1");
  await recoveryButton.click();
  await expect(recoveryButton).toHaveText("Recovered 2");

  expect(pageErrors).toEqual([]);
  expect(consoleErrors).toEqual([]);
});
