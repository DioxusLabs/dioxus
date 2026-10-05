// @ts-check
const { test, expect } = require("@playwright/test");

test("button click", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

  // Expect the page to contain the counter text.
  const main = page.locator("#main");
  await expect(main).toContainText("hello axum! 0");

  // Click the increment button.
  await page.getByRole("button", { name: "Increment" }).click();

  // Expect the page to contain the updated counter text.
  await expect(main).toContainText("hello axum! 1");
});

test("svg", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

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
  await page.goto("http://127.0.0.1:3030");

  // Expect the page to contain the div with the raw attribute.
  const div = page.locator("div.raw-attribute-div");
  await expect(div).toHaveAttribute("raw-attribute", "raw-attribute-value");
});

test("hidden attribute", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

  // Expect the page to contain the div with the hidden attribute.
  const div = page.locator("div.hidden-attribute-div");
  await expect(div).toHaveAttribute("hidden", "true");
});

test("dangerous inner html", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

  // Expect the page to contain the div with the dangerous inner html.
  const div = page.locator("div.dangerous-inner-html-div");
  await expect(div).toContainText("hello dangerous inner html");
});

test("input value", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

  // Expect the page to contain the input with the value.
  const input = page.locator("#input-value");
  await expect(input).toHaveValue("hello input");
});

test("style", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

  // Expect the page to contain the div with the style.
  const div = page.locator("div.style-div");
  await expect(div).toHaveText("colored text");
  await expect(div).toHaveCSS("color", "rgb(255, 0, 0)");
});

test("onmounted", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");

  // Expect the onmounted event to be called exactly once.
  const mountedDiv = page.locator("div.onmounted-div");
  await expect(mountedDiv).toHaveText("onmounted was called 1 times");
});

test("file picker opens the browser dialog", async ({ page }) => {
  /** @type {string[]} */
  const requests = [];
  page.on("request", (request) => {
    if (request.url().includes("__file_dialog")) requests.push(request.url());
  });
  await page.goto("http://127.0.0.1:3030");

  for (const picker of [page.locator("#file-picker"), page.locator('label[for="file-picker"]')]) {
    const chooser = page.waitForEvent("filechooser", { timeout: 3000 });
    await picker.click();
    await (await chooser).setFiles([]);
  }
  expect(requests).toEqual([]);
});

for (const id of ["file-picker", "form-file-picker"]) {
  test(`file picker uploads contents through input and change (${id})`, async ({ page }) => {
    /** @type {string[]} */
    const messages = [];
    page.on("websocket", (socket) => {
      socket.on("framesent", ({ payload }) => {
        const text = payload.toString();
        if (text.startsWith("{")) messages.push(text);
      });
    });
    await page.goto("http://127.0.0.1:3030");
    await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
    messages.length = 0;
    const picker = page.locator(`#${id}`);
    await picker.setInputFiles({
      name: "greeting.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("hello"),
    });
    const modified = await picker.evaluate((input) => /** @type {HTMLInputElement} */ (input).files[0].lastModified);
    const expected = `greeting.txt|5|text/plain|${modified}|[104, 101, 108, 108, 111]`;
    await expect(page.locator(`#${id}-input`)).toHaveText(expected);
    await expect(page.locator(`#${id}-change`)).toHaveText(expected);
    await expect(page.locator(`#${id}-values`)).toHaveText(expected);
    await expect(page.locator(`#${id}-text`)).toHaveText("hello");
    await expect(page.locator(`#${id}-counts`)).toHaveText("1,1");
    const events = messages
      .filter((message) => message !== "__ping__")
      .map((message) => JSON.parse(message))
      .filter((message) => message.method === "user_event" || message.method === "file_event")
      .map((message) => message.method === "file_event" ? message.params.event.name : message.params.name);
    expect(events).toEqual(["input", "change"]);

    if (id === "form-file-picker") {
      await expect(page.locator(`#${id}-description`)).toHaveText("upload description");
      await page.locator("#upload-form").dispatchEvent("submit");
      await expect(page.locator("#submitted-files")).toHaveText(expected);
    }

    await picker.setInputFiles([]);
    await expect(page.locator(`#${id}-counts`)).toHaveText("2,2");
    await expect(page.locator(`#${id}-input`)).toHaveText("");
    await expect(page.locator(`#${id}-change`)).toHaveText("");
    await expect(page.locator(`#${id}-values`)).toHaveText("");
  });

  test(`file picker preserves duplicate names, binary and empty files (${id})`, async ({ page }) => {
    await page.goto("http://127.0.0.1:3030");
    const picker = page.locator(`#${id}`);
    await picker.setInputFiles([
      { name: "same.bin", mimeType: "application/octet-stream", buffer: Buffer.from([0, 255, 128]) },
      { name: "same.bin", mimeType: "application/octet-stream", buffer: Buffer.from([42]) },
      { name: "empty.txt", mimeType: "text/plain", buffer: Buffer.alloc(0) },
    ]);
    const modified = await picker.evaluate((input) => Array.from(/** @type {HTMLInputElement} */ (input).files, (file) => file.lastModified));
    const expected = [
      `same.bin|3|application/octet-stream|${modified[0]}|[0, 255, 128]`,
      `same.bin|1|application/octet-stream|${modified[1]}|[42]`,
      `empty.txt|0|text/plain|${modified[2]}|[]`,
    ].join("\n");
    await expect(page.locator(`#${id}-input`)).toHaveText(expected);
    await expect(page.locator(`#${id}-change`)).toHaveText(expected);
    await expect(page.locator(`#${id}-values`)).toHaveText(expected);
    await expect(page.locator(`#${id}-counts`)).toHaveText("1,1");
  });
}

test("multiple files, including a large file, upload over HTTP", async ({ page }) => {
  let closed = false;
  let uploadMetadata;
  const uploadRequests = [];
  page.on("websocket", (socket) => {
    socket.on("framesent", ({ payload }) => {
      expect(typeof payload).toBe("string");
      if (payload.startsWith("{")) {
        const message = JSON.parse(payload);
        if (message.method === "file_event") uploadMetadata = message.params;
      }
    });
    socket.on("close", () => { closed = true; });
  });
  page.on("request", (request) => {
    if (request.url().includes("/ws/upload/")) uploadRequests.push(request);
  });
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");

  const largeSize = 17 * 1024 * 1024;
  const secondSize = 1024 * 1024;
  await page.locator("#large-file-picker").evaluate((input, { largeSize, secondSize }) => {
    const largeBytes = new Uint8Array(largeSize);
    largeBytes.fill(255);
    const secondBytes = new Uint8Array(secondSize);
    secondBytes.fill(42);
    const files = new DataTransfer();
    files.items.add(new File([largeBytes], "large.bin", { type: "application/octet-stream" }));
    files.items.add(new File([secondBytes], "second.bin", { type: "application/octet-stream" }));
    /** @type {HTMLInputElement} */ (input).files = files.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, { largeSize, secondSize });

  await expect(page.locator("#large-upload")).toHaveText([
    `large.bin|${largeSize}|255|255`,
    `second.bin|${secondSize}|42|42`,
  ].join("\n"));
  expect(closed).toBe(false);
  expect(uploadMetadata.event.data.values).toMatchObject([
    { file: { name: "large.bin", path: "", size: largeSize } },
    { file: { name: "second.bin", path: "", size: secondSize } },
  ]);
  expect(uploadRequests).toHaveLength(2);
  for (const [request, size] of [
    [uploadRequests[0], largeSize],
    [uploadRequests[1], secondSize],
  ]) {
    expect(request.method()).toBe("PUT");
    expect(new URL(request.url()).pathname).toMatch(/^\/ws\/upload\/[0-9a-f-]+$/);
    const uploadHeaders = await request.allHeaders();
    expect(uploadHeaders["content-type"]).toBe("application/octet-stream");
    expect(uploadHeaders["content-length"]).toBe(size.toString());
    expect(uploadHeaders["x-content-size"]).toBe(size.toString());
  }
});

test("file uploads work when running an existing VirtualDom", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030/?direct=true");
  await page.locator("#large-file-picker").setInputFiles({
    name: "direct.bin", mimeType: "application/octet-stream", buffer: Buffer.from([0, 255, 128]),
  });
  await expect(page.locator("#large-upload")).toHaveText("direct.bin|3|0|128");
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");
});

test("file uploads work with a cross-origin absolute WebSocket URL", async ({ page }) => {
  await page.goto("http://localhost:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");

  await page.evaluate(() => {
    const originalFetch = window.fetch;
    let uploadCredentials;
    window.fetch = (input, init) => {
      if (String(input).includes("/ws/upload/")) uploadCredentials = init?.credentials;
      return originalFetch(input, init);
    };
    Object.assign(window, { uploadCredentials: () => uploadCredentials });
  });

  await page.locator("#large-file-picker").setInputFiles({
    name: "cross-origin.bin",
    mimeType: "application/octet-stream",
    buffer: Buffer.from([42]),
  });
  await expect(page.locator("#large-upload")).toHaveText("cross-origin.bin|1|42|42");
  expect(await page.evaluate(() => /** @type {any} */ (window).uploadCredentials())).toBe("include");
});

test("retained files count toward only their connection's upload cap", async ({ page, context }) => {
  let sequence = 0;
  async function probe(page, size) {
    const name = `quota-probe-${sequence++}.bin`;
    const requests = [];
    const track = (request) => {
      if (request.url().includes("/ws/upload/")) requests.push(request);
    };
    page.on("request", track);
    await page.locator("#file-picker").evaluate((input, { size, name }) => {
      const files = new DataTransfer();
      files.items.add(new File(["x"], name));
      input.files = files.files;
      // Exercise reservations without allocating a gigabyte. Accepted reads fail the HTTP
      // size check, while rejected handles report the quota error before sending bytes.
      Object.defineProperty(input.files[0], "size", { value: size });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }, { size, name });
    await expect(page.locator("#file-picker-change")).toContainText(name);
    page.off("request", track);
    return { requests: requests.length, message: await page.locator("#file-picker-change").textContent() };
  }

  await page.goto("http://127.0.0.1:3030");
  await page.locator("#retained-file-picker").setInputFiles({
    name: "retained.txt", mimeType: "text/plain", buffer: Buffer.from("abc"),
  });
  await expect(page.locator("#retained-files")).toHaveText("1");
  const limit = 1024 * 1024 * 1024;
  expect((await probe(page, limit - 3)).requests).toBe(1);
  const rejected = await probe(page, limit - 2);
  expect(rejected.requests).toBe(0);
  expect(rejected.message).toContain("upload data limit");

  const other = await context.newPage();
  await other.goto("http://127.0.0.1:3030");
  await expect(other.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  expect((await probe(other, limit)).requests).toBe(1);
  await other.close();

  await page.getByRole("button", { name: "Release files" }).click();
  await expect(page.locator("#retained-files")).toHaveText("0");
  expect((await probe(page, limit)).requests).toBe(1);
});

test("a rejected upload leaves the connection usable for events and uploads", async ({ page }) => {
  /** @type {Error[]} */
  const unhandled = [];
  let closed = false;
  page.on("pageerror", (error) => unhandled.push(error));
  page.on("websocket", (socket) => {
    socket.on("close", () => { closed = true; });
  });
  await page.goto("http://127.0.0.1:3030");
  const picker = page.locator("#large-file-picker");
  await picker.evaluate((element) => {
    const input = /** @type {HTMLInputElement} */ (element);
    const files = new DataTransfer();
    files.items.add(new File(["x"], "too-large.bin"));
    input.files = files.files;
    // Exceed the default limit without allocating a gigabyte in the browser.
    Object.defineProperty(input.files[0], "size", { value: 1024 * 1024 * 1024 + 1 });
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });

  await expect(page.locator("#large-upload")).toContainText("exceeds the connection's upload data limit");
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");
  await picker.setInputFiles({
    name: "retry.bin", mimeType: "application/octet-stream", buffer: Buffer.from([42]),
  });
  await expect(page.locator("#large-upload")).toHaveText("retry.bin|1|42|42");
  expect(closed).toBe(false);
  expect(unhandled).toEqual([]);
});

test("invalid upload metadata does not block subsequent events or uploads", async ({ page }) => {
  /** @type {Error[]} */
  const unhandled = [];
  let closed = false;
  page.on("pageerror", (error) => unhandled.push(error));
  page.on("websocket", (socket) => {
    socket.on("close", () => { closed = true; });
  });
  await page.goto("http://127.0.0.1:3030");
  const picker = page.locator("#large-file-picker");
  await picker.evaluate((element) => {
    const input = /** @type {HTMLInputElement} */ (element);
    const files = new DataTransfer();
    // Browsers allow dates before the Unix epoch, which the server's u64 cannot represent.
    files.items.add(new File(["x"], "old-file.txt", { type: "text/plain", lastModified: -1 }));
    input.files = files.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await page.getByRole("button", { name: "Increment" }).click();

  await expect.poll(() => page.evaluate(() => window.ipc.files.size)).toBe(0);
  await expect(page.locator("#large-upload")).toHaveText("");
  await expect(page.locator("#main")).toContainText("hello axum! 1");
  expect(await page.evaluate(() => window.ipc.activeFileUploads.size)).toBe(0);
  await picker.setInputFiles({
    name: "retry.bin", mimeType: "application/octet-stream", buffer: Buffer.from([42]),
  });
  await expect(page.locator("#large-upload")).toHaveText("retry.bin|1|42|42");
  expect(closed).toBe(false);
  expect(unhandled).toEqual([]);
});

test("text edits and a later upload proceed while an earlier upload is stalled", async ({ page }) => {
  /** @type {any[]} */
  const events = [];
  page.on("websocket", (socket) => {
    socket.on("framesent", ({ payload }) => {
      const text = payload.toString();
      if (!text.startsWith("{")) return;
      const message = JSON.parse(text);
      if (message.method === "user_event") events.push(message.params);
      if (message.method === "file_event") events.push(message.params.event);
    });
  });
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  events.length = 0;

  // Delay the first HTTP upload while changing the selection and typing.
  await page.evaluate(() => {
    const original = window.ipc.uploadFile.bind(window.ipc);
    let uploads = 0;
    let release;
    const gate = new Promise((resolve) => { release = resolve; });
    Object.assign(window, {
      fileUploadCount: () => uploads,
      releaseFileUpload: () => release(),
    });
    window.ipc.uploadFile = async function (token, file, signal) {
      if (++uploads === 1) await gate;
      return original(token, file, signal);
    };

    const picker = /** @type {HTMLInputElement} */ (document.querySelector("#form-file-picker"));
    const description = /** @type {HTMLInputElement} */ (document.querySelector('input[name="description"]'));
    for (const value of ["a", "ab", "abc"]) {
      if (value !== "abc") {
        const files = new DataTransfer();
        files.items.add(new File([value], `${value}.txt`, { type: "text/plain" }));
        picker.files = files.files;
        picker.dispatchEvent(new Event("change", { bubbles: true }));
      }
      description.value = value;
      description.dispatchEvent(new Event("input", { bubbles: true }));
    }
  });
  await expect(page.locator("#description-values")).toHaveText("a,ab,abc");
  await expect(page.locator("#form-file-picker-counts")).toHaveText("0,2");
  await expect(page.locator("#form-file-picker-change")).toContainText("ab.txt|2|text/plain|");
  await expect(page.locator("#form-file-picker-change")).toContainText("[97, 98]");
  expect(events.map(({ name }) => name)).toEqual(["change", "input", "change", "input", "input"]);
  const changes = events.filter(({ name }) => name === "change");
  expect(changes.every(({ data }) => data.values.every(({ file }) => !file?.contents))).toBe(true);
  const inputs = events.filter(({ name }) => name === "input");
  expect(inputs.every(({ data }) => data.values.every(({ file }) => !file?.contents))).toBe(true);
  expect(await page.evaluate(() => /** @type {any} */ (window).fileUploadCount())).toBe(2);
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");

  await page.evaluate(() => /** @type {any} */ (window).releaseFileUpload());
  await expect(page.locator("#form-file-picker-counts")).toHaveText("0,2");
  await expect(page.locator("#form-file-picker-change")).toContainText("a.txt|1|text/plain|");
  await expect(page.locator("#form-file-picker-change")).toContainText("[97]");
});

test("empty file fields remain present in input and submit events", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");
  await page.locator('input[name="description"]').fill("empty selection");
  await expect(page.locator("#upload-selected")).toHaveText("Some(File(None))");

  await page.locator("#form-file-picker").setInputFiles({
    name: "hello.txt", mimeType: "text/plain", buffer: Buffer.from("hello"),
  });
  await expect(page.locator("#form-file-picker-counts")).toHaveText("1,1");
  await page.locator("#upload-form").dispatchEvent("submit");
  await expect(page.locator("#upload-selected")).toContainText('name: "hello.txt"');

  await page.locator("#form-file-picker").setInputFiles([]);
  await expect(page.locator("#form-file-picker-counts")).toHaveText("2,2");
  await page.locator("#upload-form").dispatchEvent("submit");
  await expect(page.locator("#upload-selected")).toHaveText("Some(File(None))");
  await expect(page.locator("#submitted-files")).toHaveText("");
});

test("ordinary LiveView send failures are reported without an uncaught error", async ({ page }) => {
  const errors = [];
  const unhandled = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => unhandled.push(error));
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  await page.evaluate(() => {
    const original = window.ipc.postMessage.bind(window.ipc);
    window.ipc.postMessage = (message) => {
      const event = JSON.parse(message);
      if (event.method === "user_event" && event.params.name === "input") {
        window.ipc.postMessage = original;
        throw new Error("IPC unavailable");
      }
      return original(message);
    };
  });
  const description = page.locator('input[name="description"]');
  await description.fill("failed edit");
  await expect.poll(() => errors.join("\n")).toContain("Failed to send LiveView event Error: IPC unavailable");
  await expect(page.locator("#description-values")).toHaveText("");
  await description.fill("successful edit");
  await expect(page.locator("#description-values")).toHaveText("successful edit");
  expect(errors).toHaveLength(1);
  expect(unhandled).toEqual([]);
});

test("the desktop interpreter applies event decisions synchronously", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  const result = await page.evaluate(() => {
    const interpreter = window.interpreter;
    const originalXHR = window.XMLHttpRequest;
    const requests = [];
    let decisions;
    // Exercise the native interpreter's Desktop branch with a synchronous host response.
    window.XMLHttpRequest = class {
      responseText = JSON.stringify({ preventDefault: true, stopPropagation: true });
      open(method, url, async) { requests.push({ method, async }); }
      setRequestHeader() {}
      send() {}
    };
    interpreter.liveview = false;
    try {
      const button = document.querySelector("button");
      button.addEventListener("click", (event) => {
        interpreter.handleEvent(event, "click", true);
        decisions = { prevented: event.defaultPrevented, stopped: event.cancelBubble };
      }, { once: true });
      button.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
      return { decisions, requests };
    } finally {
      interpreter.liveview = true;
      window.XMLHttpRequest = originalXHR;
    }
  });
  expect(result.decisions).toEqual({ prevented: true, stopped: true });
  expect(result.requests).toEqual([{ method: "POST", async: false }]);
});

test("a failed HTTP upload does not block subsequent events", async ({ page }) => {
  /** @type {string[]} */
  const errors = [];
  /** @type {Error[]} */
  const unhandled = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => unhandled.push(error));
  await page.goto("http://127.0.0.1:3030");
  await page.evaluate(() => {
    window.ipc.uploadFile = () => Promise.reject(new Error("Upload unavailable"));
  });
  await page.locator("#form-file-picker").setInputFiles({
    name: "unreadable.txt", mimeType: "text/plain", buffer: Buffer.from("hello"),
  });
  await page.locator('input[name="description"]').fill("still responsive");
  await expect(page.locator("#description-values")).toHaveText("still responsive");
  await expect(page.locator("#form-file-picker-input")).toContainText("ERROR: Error: Upload unavailable");
  await expect(page.locator("#form-file-picker-change")).toContainText("ERROR: Error: Upload unavailable");
  expect(errors.filter((message) => message.includes("Failed to send LiveView event"))).toHaveLength(0);
  expect(unhandled).toEqual([]);
});

test("a delayed upload completion does not block other events", async ({ page }) => {
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  await page.evaluate(() => {
    const send = window.ipc.postMessage.bind(window.ipc);
    let held = false;
    window.ipc.postMessage = (message) => {
      if (!held && JSON.parse(message).method === "file_upload_complete") {
        held = true;
        Object.assign(window, { releaseUploadCompletion: () => send(message) });
      } else {
        send(message);
      }
    };
  });
  await page.locator("#file-picker").setInputFiles({
    name: "hello.txt", mimeType: "text/plain", buffer: Buffer.from("hello"),
  });
  await expect.poll(() => page.evaluate(() => typeof window.releaseUploadCompletion)).toBe("function");
  await expect(page.locator("#file-picker-counts")).toHaveText("1,1");
  await expect(page.locator("#file-picker-input")).toHaveText("");
  await expect(page.locator("#file-picker-change")).toHaveText("");
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");

  await page.evaluate(() => window.releaseUploadCompletion());
  await expect(page.locator("#file-picker-input")).toContainText("[104, 101, 108, 108, 111]");
  await expect(page.locator("#file-picker-change")).toContainText("[104, 101, 108, 108, 111]");
});

test("file read requests can arrive out of order", async ({ page }) => {
  const unhandled = [];
  page.on("pageerror", (error) => unhandled.push(error));
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  await page.evaluate(() => {
    const ws = window.ipc.ws;
    const onmessage = ws.onmessage;
    let held = false;
    ws.onmessage = (message) => {
      const bytes = new Uint8Array(message.data);
      if (!held && bytes[0] === 0 &&
          new TextDecoder().decode(bytes.slice(1)).includes('"type":"file_upload"')) {
        held = true;
        Object.assign(window, { releaseFileRequest: () => onmessage(message) });
      } else {
        onmessage(message);
      }
    };
  });
  const picker = page.locator("#large-file-picker");
  await picker.setInputFiles({ name: "slow.bin", mimeType: "application/octet-stream", buffer: Buffer.from([1]) });
  await expect.poll(() => page.evaluate(() => typeof window.releaseFileRequest)).toBe("function");
  await picker.setInputFiles({ name: "fast.bin", mimeType: "application/octet-stream", buffer: Buffer.from([2]) });
  await expect(page.locator("#large-upload")).toHaveText("fast.bin|1|2|2");
  await page.evaluate(() => window.releaseFileRequest());
  await expect(page.locator("#large-upload")).toHaveText("slow.bin|1|1|1");
  await expect.poll(() => page.evaluate(() => window.ipc.activeFileUploads.size)).toBe(0);
  expect(unhandled).toEqual([]);
});

test("selection and submission retain unread files without uploading them", async ({ page }) => {
  const uploads = [];
  page.on("request", (request) => {
    if (request.url().includes("/ws/upload/")) uploads.push(request);
  });
  await page.goto("http://127.0.0.1:3030");
  await page.locator("#retained-file-picker").setInputFiles({
    name: "unread.txt", mimeType: "text/plain", buffer: Buffer.from("hello"),
  });
  await expect(page.locator("#retained-files")).toHaveText("1");
  await page.locator("#retained-form").dispatchEvent("submit");
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");
  expect(uploads).toEqual([]);
  expect(await page.evaluate(() => window.ipc.files.size)).toBe(1);
  await page.getByRole("button", { name: "Release files" }).click();
  await expect(page.locator("#retained-files")).toHaveText("0");
  await expect.poll(() => page.evaluate(() => window.ipc.files.size)).toBe(0);
});

// Resolve HTTP tokens through the browser file IDs carried by the event.
function trackUploadNames(page) {
  const namesByToken = new Map();
  page.on("websocket", (socket) => {
    const namesById = new Map();
    socket.on("framesent", ({ payload }) => {
      if (typeof payload !== "string" || !payload.startsWith("{")) return;
      const message = JSON.parse(payload);
      if (message.method === "file_event") {
        const names = message.params.event.data.values.filter((value) => value.file).map((value) => value.file.name);
        message.params.file_ids.forEach((id, index) => namesById.set(id, names[index]));
      }
    });
    socket.on("framereceived", ({ payload }) => {
      if (typeof payload === "string" || payload[0] !== 0) return;
      const text = payload.subarray(1).toString();
      if (!text.startsWith("{")) return;
      const message = JSON.parse(text);
      if (message.type === "file_upload") {
        namesByToken.set(message.data.token, namesById.get(message.data.id));
      }
    });
  });
  return (request) => namesByToken.get(new URL(request.url()).pathname.split("/").pop());
}

test("a failed file does not cancel another concurrent upload", async ({ page }) => {
  const uploadName = trackUploadNames(page);
  const errors = [];
  const unhandled = [];
  page.on("websocket", (socket) => {
    socket.on("framesent", ({ payload }) => {
      if (typeof payload !== "string" || !payload.startsWith("{")) return;
      const message = JSON.parse(payload);
      if (message.method === "file_upload_error") errors.push(message.params.error);
    });
  });
  page.on("pageerror", (error) => unhandled.push(error));
  let stalled;
  await page.route("**/ws/upload/*", async (route) => {
    const name = uploadName(route.request());
    if (name.includes("slow.bin")) {
      stalled = route;
    } else if (name.includes("failed.bin")) {
      await route.fulfill({ status: 500 });
    } else {
      await route.continue();
    }
  });
  await page.goto("http://127.0.0.1:3030");
  const picker = page.locator("#large-file-picker");
  for (const name of ["slow.bin", "failed.bin", "fast.bin"]) {
    await picker.evaluate((input, name) => {
      const files = new DataTransfer();
      files.items.add(new File([new Uint8Array([42])], name, { type: "application/octet-stream" }));
      input.files = files.files;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }, name);
  }
  await expect(page.locator("#large-upload")).toHaveText("fast.bin|1|42|42");
  await expect.poll(() => errors.join("\n")).toContain("LiveView file upload failed with status 500");
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");
  await stalled.continue();
  await expect(page.locator("#large-upload")).toHaveText("slow.bin|1|42|42");
  expect(unhandled).toEqual([]);
  expect(await page.evaluate(() => window.ipc.activeFileUploads.size)).toBe(0);
});

test("files upload as each stream is read and preserve their metadata order", async ({ page }) => {
  const uploadName = trackUploadNames(page);
  let stalled;
  let completed = 0;
  page.on("response", (response) => {
    if (response.url().includes("/ws/upload/") && response.status() === 204) completed++;
  });
  await page.route("**/ws/upload/*", (route) => {
    if (uploadName(route.request()) === "slow.bin") {
      stalled = route;
    } else {
      return route.continue();
    }
  });
  await page.goto("http://127.0.0.1:3030");
  await page.locator("#large-file-picker").evaluate((input) => {
    const files = new DataTransfer();
    files.items.add(new File([new Uint8Array([42])], "slow.bin", { type: "application/octet-stream" }));
    files.items.add(new File([new Uint8Array([128, 255])], "fast.bin", { type: "application/octet-stream" }));
    input.files = files.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await expect.poll(() => Boolean(stalled)).toBe(true);
  await page.getByRole("button", { name: "Increment" }).click();
  await expect(page.locator("#main")).toContainText("hello axum! 1");
  expect(completed).toBe(0);
  await expect(page.locator("#large-upload")).toHaveText("");
  await stalled.continue();
  await expect(page.locator("#large-upload")).toHaveText("slow.bin|1|42|42\nfast.bin|2|128|255");
});

test("closing the websocket cancels active and queued file reads", async ({ page }) => {
  const unhandled = [];
  page.on("pageerror", (error) => unhandled.push(error));
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  await page.evaluate(() => {
    const started = [], aborted = [];
    const fetch = window.fetch;
    Object.assign(window, { uploadActivity: { started, aborted } });
    window.fetch = (url, options) => {
      if (!String(url).includes("/ws/upload/")) return fetch(url, options);
      started.push(options.body.name);
      return new Promise((_, reject) => {
        options.signal.addEventListener("abort", () => {
          aborted.push(options.body.name);
          reject(options.signal.reason);
        }, { once: true });
      });
    };
    const picker = document.querySelector("#large-file-picker");
    for (let index = 0; index < 5; index++) {
      const files = new DataTransfer();
      files.items.add(new File(["x"], `file-${index}.bin`));
      picker.files = files.files;
      picker.dispatchEvent(new Event("change", { bubbles: true }));
    }
  });
  await expect.poll(() => page.evaluate(() => window.uploadActivity.started.length)).toBe(4);
  await expect.poll(() => page.evaluate(() => window.ipc.fileUploadQueue.length)).toBe(1);
  await page.evaluate(() => window.ipc.ws.close());
  await expect.poll(() => page.evaluate(() => window.uploadActivity.aborted.length)).toBe(4);
  await expect.poll(() => page.evaluate(() => window.ipc.runningFileUploads)).toBe(0);
  expect(await page.evaluate(() => ({
    started: window.uploadActivity.started.length,
    active: window.ipc.activeFileUploads.size,
    queued: window.ipc.fileUploadQueue.length,
    retained: window.ipc.files.size,
  }))).toEqual({ started: 4, active: 0, queued: 0, retained: 0 });
  expect(unhandled).toEqual([]);
});

for (const status of [200, 204]) {
  test(`an upload handled by a fallback route keeps the connection usable (${status})`, async ({ page }) => {
    const errors = [];
    const unhandled = [];
    let closed = false;
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    page.on("pageerror", (error) => unhandled.push(error));
    page.on("websocket", (socket) => {
      socket.on("close", () => { closed = true; });
    });
    // A custom router can return success without ever receiving the file into LiveView.
    await page.route("**/ws/upload/*", (route) => route.fulfill({
      status,
      contentType: "text/html",
      body: status === 200 ? "<!doctype html><html>Application fallback</html>" : "",
    }));
    await page.goto("http://127.0.0.1:3030");
    const picker = page.locator("#file-picker");
    const file = { name: "hello.txt", mimeType: "text/plain", buffer: Buffer.from("hello") };
    await picker.setInputFiles(file);
    await expect(page.locator("#file-picker-input")).toContainText("HTTP upload handler");
    await expect(page.locator("#file-picker-change")).toContainText("HTTP upload handler");
    await expect(page.locator("#file-picker-counts")).toHaveText("1,1");
    await page.getByRole("button", { name: "Increment" }).click();
    await expect(page.locator("#main")).toContainText("hello axum! 1");

    await page.unroute("**/ws/upload/*");
    await picker.setInputFiles(file);
    await expect(page.locator("#file-picker-text")).toHaveText("hello");
    await expect(page.locator("#file-picker-counts")).toHaveText("2,2");
    expect(closed).toBe(false);
    expect(unhandled).toEqual([]);
    expect(errors).toHaveLength(0);
  });
}

test("the keepalive timer stops when the websocket closes", async ({ page }) => {
  const errors = [];
  let pings = 0;
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("websocket", (socket) => {
    socket.on("framesent", ({ payload }) => {
      if (payload === "__ping__") pings++;
    });
  });
  await page.clock.install();
  await page.goto("http://127.0.0.1:3030");
  await expect(page.locator(".onmounted-div")).toHaveText("onmounted was called 1 times");
  await page.clock.runFor(30000);
  await expect.poll(() => pings).toBe(1);
  await page.evaluate(() => window.ipc.ws.close());
  await expect.poll(() => page.evaluate(() => window.ipc.ws.readyState)).toBe(3);
  await page.clock.runFor(60000);
  expect(pings).toBe(1);
  expect(errors).toEqual([]);
});
