// @ts-check
const { test, expect } = require("@playwright/test");

const URL = "http://localhost:7981";

test("ssr emits no hydration markers", async ({ page }) => {
  // Capture the raw server response before any client-side scripts run, so
  // we're asserting on the SSR output and not the live DOM.
  const res = await page.request.get(URL);
  const html = await res.text();

  // Strip scripts before scanning for comments; injected hydration scripts
  // legitimately contain serialized hydration metadata.
  const stripped = html.replace(/<script\b[\s\S]*?<\/script>/gi, "");

  expect(stripped).not.toContain("data-node-hydration");
  expect(stripped).not.toContain("<!--node-id");
  expect(stripped).not.toContain("<!--placeholder");
  expect(stripped).not.toContain("<!--#-->");
});

test("textarea with dynamic text hydrates cleanly", async ({ page }) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  const value = await page
    .locator("#user-textarea")
    .evaluate((el) => /** @type {HTMLTextAreaElement} */ (el).value);
  expect(value).toBe("hello & world");
});

test("dangerous inner html hydrates host and updates innerHTML", async ({
  page,
}) => {
  const res = await page.request.get(URL);
  const html = await res.text();
  expect(html).toContain(
    `<p id="raw-inner-child">raw <strong>HTML</strong></p>`
  );

  await page.goto(URL);
  await page.waitForTimeout(2000);

  const host = page.locator("#raw-inner-host");
  await expect(host.locator("#raw-inner-child")).toHaveCount(1);
  await expect(host).toContainText("raw HTML");

  await page.locator("#swap-raw-inner").click();
  await expect(host.locator("#raw-inner-child")).toHaveCount(0);
  await expect(host.locator("#raw-inner-child-updated")).toHaveCount(1);
  await expect(host).toContainText("changed HTML");
});

// Adjacent dynamic texts merge into one DOM text node during SSR; the walker
// must split it so each dynamic slice owns its own node. Split offsets are
// UTF-16 code units (matching JS `Text.splitText`) — splitting mid surrogate
// pair would corrupt non-BMP text.
test("adjacent dynamic texts split correctly after hydration", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  const div = page.locator("#adjacent-texts");
  await expect(div).toHaveText("AAABBB");

  await page.locator("#swap-adjacent").click();
  // After swap: a="" and b="CCC". Visible text must be exactly "CCC".
  await expect(div).toHaveText("CCC");

  const utf16 = page.locator("#utf16-text");
  await expect(utf16).toHaveText("before 💧 | 🌊🌊 after");
  await page.locator("#utf16-swap").click();
  // a is now "é💧é" (4 utf16 units) and b is "" → "before é💧é |  after"
  await expect(utf16).toHaveText("before é💧é |  after");
});

// Empty dynamic texts in every position of a text run: long runs
// (trailing/leading/all-empty), an empty sandwiched between non-empties
// (addressable via `SynthText` between two `SplitText` cursor moves), and
// empties separated by static text — all must hydrate in source order and
// stay individually addressable.
test("empty dynamic texts hydrate in source order in every position", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  const trailing = page.locator("#trailing-empties-10");
  const leading = page.locator("#leading-empties-10");
  const allEmpty = page.locator("#all-empties-10");
  await expect(trailing).toHaveText("HEAD");
  await expect(leading).toHaveText("TAIL");
  await expect(allEmpty).toHaveText("");

  await page.locator("#fill-runs").click();
  const labels = "[a][b][c][d][e][f][g][h][i][j]";
  await expect(trailing).toHaveText("HEAD" + labels);
  await expect(leading).toHaveText(labels + "TAIL");
  await expect(allEmpty).toHaveText(labels);

  const middle = page.locator("#empty-middle");
  await expect(middle).toHaveText("AAABBB");
  await page.locator("#fill-middle").click();
  await expect(middle).toHaveText("leftMIDright");

  const separated = page.locator("#separated-empty-slots");
  await expect(separated).toHaveText("S");
  await page.locator("#fill-separated-slot-b").click();
  await expect(separated).toHaveText("SB");
  await page.locator("#fill-separated-slot-a").click();
  await expect(separated).toHaveText("ASB");
});


// A pure-text child component flattened into the parent's text-run. SSR
// emits a single merged text node; the walker must split it so the child
// owns only its dynamic slice. Otherwise the child's later `set_text`
// would either no-op (slice unmapped) or overwrite the entire merged text
// (mapped to the wrong node).
test("child component contributing to parent text-run hydrates correctly", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  const div = page.locator("#component-in-run");
  await expect(div).toHaveText("before MID after");

  await page.locator("#swap-component-text").click();
  await expect(div).toHaveText("before UPDATED after");
});

// A virtual placeholder (SSR rendered nothing for the `None` branch) becomes
// real content via `replace_with`. The interpreter's anchor op must resolve
// against the virtual entry's `{parent, after}` instead of calling
// `.replaceWith` on a synthesized comment.
// Virtual placeholders never materialize comment anchors: an empty slot is
// replaced with content and back (diff-time `create_placeholder` stays
// virtual), `insert_after` against a trailing virtual placeholder advances
// its `after` pointer per append, and `remove(id)` collapsing a hydrated
// element back to an empty slot injects no comment either.
test("virtual placeholders anchor replace/insert/remove without comments", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  const toContent = page.locator("#placeholder-to-content");
  await expect(toContent).toHaveText("before  after");
  // No `<span#placeholder-content>` and crucially no `<!---->` anchor sitting
  // between the static text contributions.
  expect(await toContent.evaluate((el) => el.innerHTML)).not.toContain("<!--");
  await page.locator("#toggle-placeholder").click();
  await expect(toContent).toHaveText("before HELLO after");
  await expect(page.locator("#placeholder-content")).toBeVisible();
  await page.locator("#toggle-placeholder").click();
  await expect(toContent).toHaveText("before  after");
  expect(await toContent.evaluate((el) => el.innerHTML)).not.toContain("<!--");

  const trailing = page.locator("#trailing-placeholder");
  await expect(trailing).toHaveText("HEAD");
  const button = page.locator("#append-trailing");
  await button.click();
  await expect(trailing).toHaveText("HEAD(1)[1]");
  await button.click();
  await expect(trailing).toHaveText("HEAD(2)[1][2]");
  await button.click();
  await expect(trailing).toHaveText("HEAD(3)[1][2][3]");

  const removable = page.locator("#remove-placeholder");
  await expect(removable).toHaveText("edges PRESENT edges");
  await page.locator("#hide-removable").click();
  await expect(removable).toHaveText("edges  edges");
  expect(await removable.evaluate((el) => el.innerHTML)).not.toContain("<!--");
});

test("parser-inserted wrapper does not capture hydrated row state", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  const row = page.locator("#parser-row");
  const wrapper = page.locator("#parser-table tbody");
  await expect(wrapper).toHaveCount(1);
  await expect(row).toHaveClass("plain");

  await expect(row).toHaveAttribute("data-dioxus-id", /\d+/);
  await expect(wrapper).not.toHaveAttribute("data-dioxus-id", /\d+/);

  await row.click();
  await expect(page.locator("#parser-row-clicks")).toHaveText("row clicks: 1");

  await page.locator("#mark-parser-row").click();
  await expect(row).toHaveClass("marked");
  await expect(wrapper).not.toHaveClass(/marked/);
});

test("trailing root-level placeholder keeps the mount parent", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  await expect(page.locator("#trailing-root-late")).toHaveCount(0);
  await page.locator("#show-trailing-root").click();
  await expect(page.locator("#trailing-root-late")).toHaveText(
    "late trailing root"
  );

  const order = await page.evaluate(() =>
    Array.from(document.querySelectorAll("button, div"))
      .filter((el) =>
        [
          "show-trailing-root",
          "trailing-root-before",
          "trailing-root-late",
        ].includes(el.id)
      )
      .map((el) => el.id)
  );
  expect(order).toEqual([
    "show-trailing-root",
    "trailing-root-before",
    "trailing-root-late",
  ]);
});

test("svg elements can receive hydrated listeners", async ({ page }) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  await expect(page.locator("#svg-click-count")).toHaveText("svg clicks: 0");
  await page.locator("#hydrated-circle").click();
  await expect(page.locator("#svg-click-count")).toHaveText("svg clicks: 1");
  await expect(page.locator("#hydrated-circle")).toHaveAttribute(
    "data-dioxus-id",
    /\d+/
  );
});

// Whole-page guarantee: after hydration and one round of dynamic mutations,
// no comment nodes have been introduced for placeholders. Template-baseline
// comments are out of scope for this regression (those come from
// `create_template_node` on CSR-cloned templates), but the harness renders
// SSR-first, so any comment we see was injected by hydration or by an
// anchor op.
test("hydration and dynamic ops leave no comment or empty-text markers", async ({
  page,
}) => {
  await page.goto(URL);
  await page.waitForTimeout(2000);

  // Trigger every dynamic op that interacts with a placeholder, including
  // toggle-back-to-empty paths that exercise the CSR template-clone +
  // diff-time `create_placeholder` flow.
  await page.locator("#toggle-placeholder").click();
  await page.locator("#toggle-placeholder").click();
  await page.locator("#append-trailing").click();
  await page.locator("#hide-removable").click();
  await page.waitForTimeout(200);

  const counts = await page.evaluate(() => {
    let comments = 0;
    const emptyTextLocations = [];
    const walker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_COMMENT | NodeFilter.SHOW_TEXT
    );
    while (walker.nextNode()) {
      const n = walker.currentNode;
      if (n.nodeType === Node.COMMENT_NODE) comments++;
      else if (
        n.nodeType === Node.TEXT_NODE &&
        /** @type {Text} */ (n).data === ""
      ) {
        const parent = n.parentElement;
        emptyTextLocations.push(
          parent
            ? `${parent.tagName.toLowerCase()}#${parent.id || "(no-id)"}`
            : "(detached)"
        );
      }
    }
    return { comments, emptyTextLocations };
  });
  expect(counts.comments).toBe(0);
  expect(counts.emptyTextLocations).toEqual([]);
});
