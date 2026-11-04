// @ts-check
const { test, expect } = require("@playwright/test");
const fs = require("fs");
const path = require("path");

const coreSource = fs.readFileSync(
  path.resolve(__dirname, "..", "interpreter", "src", "js", "core.js"),
  "utf8"
);
const nativeSource = fs.readFileSync(
  path.resolve(__dirname, "..", "interpreter", "src", "js", "native.js"),
  "utf8"
);

test("native interpreter intercepts external links before event responses", async ({ page }) => {
  await page.setContent(`
    <div id="root">
      <a id="external" href="https://external.example/path">External</a>
      <a id="internal" href="#internal">Internal</a>
    </div>
  `);

  const result = await page.evaluate(
    async ({ coreSource, nativeSource }) => {
      const messages = [];
      globalThis.ipc = {
        postMessage(message) {
          messages.push(JSON.parse(message));
        },
      };

      const coreUrl = URL.createObjectURL(
        new Blob([coreSource], { type: "text/javascript" })
      );
      const nativeUrl = URL.createObjectURL(
        new Blob([nativeSource], { type: "text/javascript" })
      );

      try {
        const { BaseInterpreter } = await import(coreUrl);
        globalThis.RawInterpreter = BaseInterpreter;
        const { NativeInterpreter } = await import(nativeUrl);
        const interpreter = new NativeInterpreter("https://app.example", false);

        // An undefined response reproduces the path that previously skipped link handling.
        interpreter.sendSerializedEvent = () => undefined;

        const root = document.querySelector("#root");
        interpreter.initialize(root);
        interpreter.createListener("click", root, true);

        function clickLink(id) {
          const link = document.querySelector(`#${id}`);
          let defaultPrevented = false;
          root.addEventListener(
            "click",
            (event) => {
              defaultPrevented = event.defaultPrevented;
            },
            { once: true }
          );

          link.dispatchEvent(
            new MouseEvent("click", { bubbles: true, cancelable: true })
          );

          return { defaultPrevented, messages: messages.splice(0) };
        }

        return {
          external: clickLink("external"),
          internal: clickLink("internal"),
        };
      } finally {
        URL.revokeObjectURL(coreUrl);
        URL.revokeObjectURL(nativeUrl);
      }
    },
    { coreSource, nativeSource }
  );

  expect(result.external).toEqual({
    defaultPrevented: true,
    messages: [
      {
        method: "browser_open",
        params: { href: "https://external.example/path" },
      },
    ],
  });
  expect(result.internal).toEqual({
    defaultPrevented: false,
    messages: [],
  });
  expect(await page.evaluate(() => location.href)).toBe("about:blank#internal");
});
