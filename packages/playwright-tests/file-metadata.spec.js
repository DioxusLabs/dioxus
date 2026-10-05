const { test, expect } = require("@playwright/test");
const fs = require("node:fs");
const path = require("node:path");

const interpreterUrl = "data:text/javascript;base64," + fs.readFileSync(
  path.join(__dirname, "../interpreter/src/js/native.js")
).toString("base64");

test.beforeEach(async ({ page }) => {
  await page.setContent(`
    <form data-dioxus-id="1">
      <input name="description" value="upload">
      <input id="first" name="first" type="file" multiple data-dioxus-id="2">
      <input id="second" name="second" type="file" data-dioxus-id="3">
    </form>
  `);
  await page.evaluate(async (url) => {
    // Exercise the shipped event interpreter without a renderer or native host.
    window.RawInterpreter = class { initialize() {} };
    const { NativeInterpreter } = await import(url);
    const interpreter = window.interpreter = new NativeInterpreter("http://native", true);
    interpreter.initialize(document.body);
    window.sent = [];
    interpreter.sendSerializedEvent = (body) => {
      window.sent.push(structuredClone(body));
    };
    window.emit = (target, event) => {
      target.addEventListener(event.type, (event) => {
        event.preventDefault();
        interpreter.handleEvent(event, event.type, event.bubbles);
      }, { once: true });
      target.dispatchEvent(event);
      return window.sent.at(-1).data;
    };
  }, interpreterUrl);
});

test("browser files have names and empty paths", async ({ page }, testInfo) => {
  const directory = testInfo.outputPath("uploads");
  fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(path.join(directory, "résumé.txt"), "hello");
  fs.writeFileSync(path.join(directory, "empty.txt"), "");
  await page.locator("#first").evaluate((input) => {
    input.setAttribute("webkitdirectory", "");
  });
  await page.locator("#first").setInputFiles(directory);

  const result = await page.evaluate(() => {
    window.interpreter.liveview = true;
    const form = document.querySelector("form");
    const input = document.querySelector("#first");
    const transfer = new DataTransfer();
    for (const file of input.files) transfer.items.add(file);
    const submitted = window.emit(form, new Event("submit", { cancelable: true }));
    const dropped = window.emit(form, new DragEvent("drop", { dataTransfer: transfer }));
    const paste = new ClipboardEvent("paste");
    // Firefox ignores clipboardData in the constructor for synthetic paste events.
    Object.defineProperty(paste, "clipboardData", { value: transfer });
    const pasted = window.emit(form, paste);
    input.value = "";
    const cleared = window.emit(form, new Event("submit", { cancelable: true }));
    return { submitted, dropped, pasted, cleared };
  });

  const files = result.submitted.values.filter((value) => value.file).map((value) => value.file);
  expect(files).toHaveLength(2);
  expect(files.map(({ name, path, size }) => ({ name, path, size }))).toEqual(expect.arrayContaining([
    { name: "résumé.txt", path: "", size: 5 },
    { name: "empty.txt", path: "", size: 0 },
  ]));
  expect(result.dropped.files.map((value) => value.file)).toEqual(files);
  expect(result.dropped.data_transfer.files).toEqual(files);
  expect(result.pasted.data_transfer.files).toEqual(files);
  expect(result.cleared.values).toEqual([
    { key: "description", text: "upload" }, { key: "first" }, { key: "second" },
  ]);
});

test("desktop events retain native metadata through selection changes", async ({ page }) => {
  await page.evaluate(() => {
    window.requests = [];
    window.dialogFiles = [];
    window.interpreter.fetchAgainstHost = async (_path, request) => {
      window.requests.push(structuredClone(request));
      return {
        json: async () => ({
          values: [
            ...request.values.filter((value) => value.key !== request.target_name),
            ...window.dialogFiles.map((file) => ({ key: request.target_name, file })),
          ],
        }),
      };
    };
  });

  const first = {
    name: "report.txt", path: "/documents/report.txt", size: 42,
    last_modified: 123, content_type: "text/plain",
  };
  const second = { ...first, path: "/archives/report.txt", size: 84 };
  const replacement = { ...first, path: "/revised/report.txt", size: 126 };

  async function select(input, metadata) {
    await page.evaluate((metadata) => { window.dialogFiles = [metadata]; }, metadata);
    await page.locator(input).click();
    await expect.poll(() => page.evaluate(() => window.sent.at(-1)?.data.values))
      .toContainEqual({ key: input.slice(1), file: metadata });
  }

  async function submit() {
    return page.evaluate(() => window.emit(
      document.querySelector("form"), new Event("submit", { cancelable: true })
    ));
  }

  await select("#first", first);
  await select("#second", second);
  expect(await page.evaluate(() => window.requests[1].values))
    .toContainEqual({ key: "first", file: first });
  expect(await page.locator("#first").evaluate((input) => ({
    name: input.files[0].name, size: input.files[0].size,
  }))).toEqual({ name: "report.txt", size: 0 });
  expect((await submit()).values).toEqual([
    { key: "description", text: "upload" },
    { key: "first", file: first },
    { key: "second", file: second },
  ]);

  const transfers = await page.evaluate(() => {
    const form = document.querySelector("form");
    const transfer = new DataTransfer();
    // Copy the dialog's File objects to another browser API, reversing input order.
    transfer.items.add(document.querySelector("#second").files[0]);
    transfer.items.add(document.querySelector("#first").files[0]);
    // Files created in the browser still use their own metadata.
    transfer.items.add(new File(["browser"], "browser.txt", {
      type: "text/plain", lastModified: 456,
    }));
    const dropped = window.emit(form, new DragEvent("drop", { dataTransfer: transfer }));
    const paste = new ClipboardEvent("paste");
    Object.defineProperty(paste, "clipboardData", { value: transfer });
    const pasted = window.emit(form, paste);
    return { dropped, pasted };
  });
  const expected = [second, first, {
    name: "browser.txt", path: "", size: 7, last_modified: 456, content_type: "text/plain",
  }];
  expect(transfers.dropped.files.map((value) => value.file)).toEqual(expected);
  expect(transfers.dropped.data_transfer.files).toEqual(expected);
  expect(transfers.pasted.data_transfer.files).toEqual(expected);

  await select("#first", replacement);
  expect((await submit()).values).toContainEqual({ key: "first", file: replacement });
  await page.locator("#first").evaluate((input) => { input.value = ""; });
  expect((await submit()).values).toEqual([
    { key: "description", text: "upload" }, { key: "first" }, { key: "second", file: second },
  ]);
  await page.locator("form").evaluate((form) => form.reset());
  expect((await submit()).values).toEqual([
    { key: "description", text: "upload" }, { key: "first" }, { key: "second" },
  ]);
});
