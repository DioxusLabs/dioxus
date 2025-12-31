// @ts-check
const { test, expect, defineConfig } = require("@playwright/test");

test("button click", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the counter text.
  const main = page.locator("#main");
  await expect(main).toContainText("hello axum! 0");
  // Expect the title to contain the counter text.
  await expect(page).toHaveTitle("hello axum! 0");

  // Click the increment button.
  let button = page.locator("button.increment-button");
  await button.click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText("hello axum! 1");
  // Expect the title to contain the updated counter text.
  await expect(page).toHaveTitle("hello axum! 1");
});

test("svg", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the svg.
  const svg = page.locator("svg");

  // Expect the svg to contain the circle.
  const circle = svg.locator("circle");
  await expect(circle).toHaveAttribute("cx", "50");
  await expect(circle).toHaveAttribute("cy", "50");
  await expect(circle).toHaveAttribute("r", "40");
  await expect(circle).toHaveAttribute("stroke", "green");
  await expect(circle).toHaveAttribute("fill", "yellow");
});

test("raw attribute", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the div with the raw attribute.
  const div = page.locator("div.raw-attribute-div");
  await expect(div).toHaveAttribute("raw-attribute", "raw-attribute-value");
});

test("hidden attribute", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the div with the hidden attribute.
  const div = page.locator("div.hidden-attribute-div");
  await expect(div).toHaveAttribute("hidden", "true");
});

test("dangerous inner html", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the div with the dangerous inner html.
  const div = page.locator("div.dangerous-inner-html-div");
  await expect(div).toContainText("hello dangerous inner html");
});

test("input value", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the input with the value.
  const input = page.locator("input");
  await expect(input).toHaveValue("hello input");
});

test("style", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the div with the style.
  const div = page.locator("div.style-div");
  await expect(div).toHaveText("colored text");
  await expect(div).toHaveCSS("color", "rgb(255, 0, 0)");
});

test("eval", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the div with the eval and have no text.
  const div = page.locator("div.eval-result");
  await expect(div).toHaveText("");

  // Click the button to run the eval.
  let button = page.locator("button.eval-button");
  await button.click();

  // Check that the title changed.
  await expect(page).toHaveTitle("Hello from Dioxus Eval!");

  // Check that the div has the eval value.
  await expect(div).toHaveText("returned eval value");
});

test("prevent default", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the page to contain the div with the eval and have no text.
  const a = page.locator("a.prevent-default");
  await expect(a).toHaveText("View source");

  // Click the <a> element to change the text
  await a.click();

  // Check that the <a> element changed.
  await expect(a).toHaveText("Psych!");
});

test("onmounted", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Expect the onmounted event to be called exactly once.
  const mountedDiv = page.locator("div.onmounted-div");
  await expect(mountedDiv).toHaveText("onmounted was called 1 times");
});

test("web-sys closure", async ({ page }) => {
  await page.goto("http://localhost:9990");
  // wait until the div is mounted
  const scrollDiv = page.locator("div#web-sys-closure-div");
  await scrollDiv.waitFor({ state: "attached" });
  await page.keyboard.press("Enter");
  await expect(scrollDiv).toHaveText("the keydown event was triggered");
});

test("document elements", async ({ page }) => {
  await page.goto("http://localhost:9990");
  // wait until the meta element is mounted
  const meta = page.locator("meta#meta-head[name='testing']");
  await meta.waitFor({ state: "attached" });
  await expect(meta).toHaveAttribute("data", "dioxus-meta-element");

  const link = page.locator("link#link-head[rel='stylesheet']");
  await link.waitFor({ state: "attached" });
  await expect(link).toHaveAttribute(
    "href",
    "https://fonts.googleapis.com/css?family=Roboto+Mono"
  );

  const stylesheet = page.locator("link#stylesheet-head[rel='stylesheet']");
  await stylesheet.waitFor({ state: "attached" });
  await expect(stylesheet).toHaveAttribute(
    "href",
    "https://fonts.googleapis.com/css?family=Roboto:300,300italic,700,700italic"
  );

  const script = page.locator("script#script-head");
  await script.waitFor({ state: "attached" });
  await expect(script).toHaveAttribute("async", "true");

  const style = page.locator("style#style-head");
  await style.waitFor({ state: "attached" });
  const main = page.locator("#main");
  await expect(main).toHaveCSS("font-family", "Roboto");
});

test("link preload and stylesheet with same href are not deduplicated", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Both links should exist (different rel = not deduplicated)
  await expect(page.locator("link#dedup-preload[rel='preload']")).toHaveCount(1);
  await expect(page.locator("link#dedup-stylesheet[rel='stylesheet']")).toHaveCount(1);
});

test("links with same href and rel are deduplicated", async ({ page }) => {
  await page.goto("http://localhost:9990");

  // Only first link should exist (same rel = deduplicated)
  await expect(page.locator("link#dedup-first")).toHaveCount(1);
  await expect(page.locator("link#dedup-second")).toHaveCount(0);
});

test("merge styles", async ({ page }) => {
  await page.goto("http://localhost:9990");
  // wait until the div is mounted
  const div = page.locator("div#merge-styles-div");
  await div.waitFor({ state: "attached" });
  await expect(div).toHaveCSS("background-color", "rgb(255, 0, 0)");
  await expect(div).toHaveCSS("width", "100px");
  await expect(div).toHaveCSS("height", "100px");
});

test("select multiple", async ({ page }) => {
  await page.goto("http://localhost:9990");
  // wait until the select element is mounted
  const staticSelect = page.locator("select#static-multiple-select");
  await staticSelect.waitFor({ state: "attached" });
  await expect(staticSelect).toHaveValues([]);
  // Make sure the multiple attribute is actually set
  await staticSelect.selectOption(["1", "2"]);
  await expect(staticSelect).toHaveValues(["1", "2"]);

  // The dynamic select element should act exactly the same
  const dynamicSelect = page.locator("select#dynamic-multiple-select");
  await dynamicSelect.waitFor({ state: "attached" });
  await expect(dynamicSelect).toHaveValues([]);
  await dynamicSelect.selectOption(["1", "2"]);
  await expect(dynamicSelect).toHaveValues(["1", "2"]);
});
