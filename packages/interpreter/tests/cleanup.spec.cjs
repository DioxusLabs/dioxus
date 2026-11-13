const { test, expect } = require("@playwright/test");
const fs = require("node:fs");
const path = require("node:path");

const interpreter = path.join(__dirname, "..");
const core = fs.readFileSync(path.join(interpreter, "src/js/core.js"), "utf8");
test.beforeEach(async ({ page }) => {
  await page.setContent('<div id="root"></div>');
  await page.evaluate(
    async ({ core }) => {
      const { BaseInterpreter } = await import(
        "data:text/javascript;base64," + btoa(core)
      );
      class Interpreter extends BaseInterpreter {}
      window.interpreter = new Interpreter();
      window.delivered = [];
      window.interpreter.initialize(document.getElementById("root"), (event) =>
        window.delivered.push([
          event.type,
          event.target.getAttribute("data-dioxus-id"),
        ]),
      );
      const i = window.interpreter;
      // Keep real browser observation behavior, with explicit target/call recording
      // and deterministic delivery of a notification queued before retirement.
      for (const name of ["ResizeObserver", "IntersectionObserver"]) {
        const Native = window[name];
        window[name] = class extends Native {
          constructor(callback) {
            super(callback);
            this.deliver = callback;
            this.targets = new Set();
            this.unobserved = [];
          }
          observe(target, options) {
            this.targets.add(target);
            super.observe(target, options);
          }
          unobserve(target) {
            this.targets.delete(target);
            this.unobserved.push(target);
            super.unobserve(target);
          }
        };
      }
      window.element = (tag, id, parent = i.root) => {
        const node = document.createElement(tag);
        parent.append(node);
        i.setNode(id, node);
        return node;
      };
      window.remove = (id) => {
        i.pushId(id);
        i.removeTop();
      };
      window.replace = (id, operands) => {
        i.pushId(id);
        for (const operand of operands) i.pushId(operand);
        i.replaceTopWith(operands.length);
      };
      window.listen = (id, name, bubbles = false) =>
        i.setNodeListener(id, name, bubbles);
    },
    { core },
  );
});

test("retirement clears nested HTML, SVG, text and comment IDs", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const p = element("div", 1);
    p.innerHTML = "<svg><g></g></svg><span>text</span><!--slot-->";
    [
      p.querySelector("g"),
      p.querySelector("span").firstChild,
      p.lastChild,
    ].forEach((n, j) => i.setNode(j + 2, n));
    remove(1);
    return (
      [1, 2, 3, 4].every((id) => i.nodes[id] === undefined) &&
      i.nodes[0] === i.root
    );
  });
  expect(result).toEqual(true);
});

test("bound detached prototypes and pending clones remain available", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    i.createElementTop("section", null);
    i.setId(10);
    const prototype = i.nodes[10];
    prototype.innerHTML = "<b>template</b>";
    i.cloneTop();
    i.setId(11);
    const pending = i.nodes[11];
    const p = element("div", 1);
    remove(1);
    return (
      i.nodes[10] === prototype &&
      i.nodes[11] === pending &&
      i.stack.at(-1)[0] === pending &&
      !pending.isConnected
    );
  });
  expect(result).toEqual(true);
});

test("moving replacement descendants preserves listeners and observation", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const p = element("div", 1),
      c = element("button", 2, p);
    element("span", 3, p);
    listen(2, "focus");
    listen(2, "resize");
    replace(1, [2]);
    c.dispatchEvent(new Event("focus"));
    return [
      i.nodes[1] === undefined,
      i.nodes[3] === undefined,
      i.root.firstChild === c,
      i.resizeObserver.targets.has(c),
      window.delivered,
    ];
  });
  expect(result).toEqual([true, true, true, true, [["focus", "2"]]]);
});

test("a rebound slot survives retirement of its previous physical node", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const p = element("div", 1),
      old = element("button", 2, p);
    listen(2, "click", true);
    listen(2, "focus");
    listen(2, "resize");
    const fresh = element("button", 2);
    listen(2, "click", true);
    listen(2, "focus");
    listen(2, "resize");
    remove(1);
    old.dispatchEvent(new Event("focus"));
    fresh.dispatchEvent(new Event("focus"));
    return [
      i.nodes[2] === fresh,
      i.global.click.active,
      i.resizeObserver.targets.has(fresh),
      i.resizeObserver.unobserved.includes(old),
      window.delivered,
    ];
  });
  expect(result).toEqual([true, 1, true, true, [["focus", "2"]]]);
});

test("duplicate listeners keep routing until their final registration is removed", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const n = element("button", 1);
    listen(1, "focus");
    listen(1, "focus");
    listen(1, "click", true);
    i.pushId(1);
    i.removeTopEventListener("focus", false);
    n.dispatchEvent(new Event("focus"));
    i.removeTopEventListener("click", true);
    const route = n.getAttribute("data-dioxus-id");
    i.removeTopEventListener("focus", false);
    i.pop();
    return [
      route,
      n.getAttribute("data-dioxus-id"),
      Object.keys(i.global),
      window.delivered,
    ];
  });
  expect(result).toEqual(["1", null, [], [["focus", "1"]]]);
});

test("retirement unregisters observers and rejects queued notifications", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const p = element("div", 1),
      n = element("button", 2, p);
    listen(2, "resize");
    listen(2, "visible");
    remove(1);
    i.resizeObserver.deliver([{ target: n }]);
    i.intersectionObserver.deliver([{ target: n }]);
    return [
      i.resizeObserver.targets.size,
      i.intersectionObserver.targets.size,
      window.delivered,
    ];
  });
  expect(result).toEqual([0, 0, []]);
});

test("explicit observer removal detaches handlers without removing remaining routing", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const n = element("button", 1);
    listen(1, "resize");
    listen(1, "visible");
    listen(1, "focus");
    i.pushId(1);
    i.removeTopEventListener("resize", false);
    i.removeTopEventListener("visible", false);
    i.pop();
    i.resizeObserver.deliver([{ target: n }]);
    i.intersectionObserver.deliver([{ target: n }]);
    n.dispatchEvent(new Event("focus"));
    return [
      i.resizeObserver.targets.size,
      i.intersectionObserver.targets.size,
      n.getAttribute("data-dioxus-id"),
      window.delivered,
    ];
  });
  expect(result).toEqual([0, 0, "1", [["focus", "1"]]]);
});

test("markerless hydration bindings retire SVG, text and empty anchors", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    i.root.innerHTML = "<svg><g></g></svg><p>merged text</p><!--empty-->";
    const svg = i.root.firstChild,
      g = svg.firstChild,
      text = i.root.children[1].firstChild,
      empty = i.root.lastChild;
    [svg, g, text, empty].forEach((n, j) => i.setNode(j + 1, n));
    i.setNodeListener(2, "click", true);
    remove(1);
    remove(3);
    remove(4);
    return (
      [1, 2, 3, 4].every((id) => i.nodes[id] === undefined) &&
      Object.keys(i.global).length === 0
    );
  });
  expect(result).toEqual(true);
});

test("positional child and popId bindings both retire", async ({ page }) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const p = element("div", 1);
    p.innerHTML = "<span>text</span>";
    i.pushId(1);
    i.child(0);
    i.setId(2);
    i.child(0);
    i.popId(3);
    remove(1);
    return [1, 2, 3].every((id) => i.nodes[id] === undefined);
  });
  expect(result).toEqual(true);
});

test("keyed-style moves preserve identity and event handlers", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const a = element("button", 1),
      b = element("button", 2),
      c = element("button", 3);
    listen(1, "focus");
    listen(1, "resize");
    i.pushId(3);
    i.pushId(1);
    i.insertAfterTop(1);
    i.pop();
    a.dispatchEvent(new Event("focus"));
    return [
      [...i.root.childNodes].map((n) => [a, b, c].indexOf(n)),
      i.resizeObserver.targets.has(a),
      window.delivered,
      i.stack.length,
    ];
  });
  expect(result).toEqual([[1, 2, 0], true, [["focus", "1"]], 1]);
});

test("mounted queues follow setId rather than an earlier pushId", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const n = element("button", 1);
    i.pushId(1);
    i.setId(2);
    i.queueTopMounted();
    i.pop();
    return [...i.takeMountedIds()];
  });
  expect(result).toEqual([2]);
});

test("mounted queues discard retired and rebound physical nodes", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    element("button", 1);
    i.pushId(1);
    i.queueTopMounted();
    i.removeTop();
    const fresh = element("button", 1);
    i.queueMounted(1);
    return [
      [...i.takeMountedIds()],
      i.nodes[1] === fresh,
      [...i.takeMountedIds()],
    ];
  });
  expect(result).toEqual([[1], true, []]);
});

test("native mounted registrations preserve routing after other listener removal", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const n = element("button", 1);
    window.ipc = { postMessage() {} };
    i.sendSerializedEvent = (value) => JSON.stringify(value);
    i.pushId(1);
    i.addTopForeignEventListener("mounted", false);
    i.addTopForeignEventListener("focus", false);
    i.removeTopEventListener("focus", false);
    const route = n.getAttribute("data-dioxus-id");
    i.removeTop();
    return [route, n.getAttribute("data-dioxus-id"), i.nodes[1] === undefined];
  });
  expect(result).toEqual(["1", null, true]);
});

test("alternating subtree sizes restore binding and delegation counts", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    for (let cycle = 0; cycle < 30; cycle++) {
      const p = element("div", 1);
      for (let j = 0; j < (cycle % 2 ? 30 : 3); j++) {
        element("button", j + 2, p);
        listen(j + 2, "click", true);
      }
      remove(1);
    }
    return [
      i.nodes.filter(Boolean).length,
      Object.keys(i.global),
      i.stack.length,
    ];
  });
  expect(result).toEqual([1, [], 1]);
});

test("self and self plus sibling replacements preserve live nodes", async ({
  page,
}) => {
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    const n = element("button", 1);
    listen(1, "focus");
    listen(1, "resize");
    replace(1, [1]);
    element("span", 2);
    replace(1, [1, 2]);
    n.dispatchEvent(new Event("focus"));
    return [
      i.root.firstChild === n,
      n.nextSibling === i.nodes[2],
      i.resizeObserver.targets.has(n),
      window.delivered,
      i.stack.length,
    ];
  });
  expect(result).toEqual([true, true, true, [["focus", "1"]], 1]);
});
async function runCompiledFixture(page, name) {
  const directory = path.resolve(
    __dirname,
    "../../../target/interpreter-cleanup-fixtures",
  );
  const raw = fs.readFileSync(
    path.join(directory, "raw-interpreter.js"),
    "utf8",
  );
  const native = fs.readFileSync(
    path.join(directory, "native-interpreter.js"),
    "utf8",
  );
  const bytes = JSON.parse(
    fs.readFileSync(path.join(directory, "fixtures.json"), "utf8"),
  )[name];
  await page.evaluate(
    async ({ raw, native, bytes }) => {
      const { RawInterpreter } = await import(
        "data:text/javascript;base64," + btoa(raw)
      );
      window.RawInterpreter = RawInterpreter;
      const { NativeInterpreter } = await import(
        "data:text/javascript;base64," + btoa(native)
      );
      const i = (window.interpreter = new NativeInterpreter("", true));
      i.initialize(document.getElementById("root"));
      i.handler = (event) =>
        window.delivered.push([
          event.type,
          event.target.getAttribute("data-dioxus-id"),
        ]);
      window.ipc = { postMessage() {} };
      i.run_from_bytes(bytes);
    },
    { raw, native, bytes },
  );
}

test("compiled binary interpreter preserves self and sibling replacement operands", async ({
  page,
}) => {
  await runCompiledFixture(page, "self");
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    window.delivered = [];
    i.nodes[1].dispatchEvent(new Event("focus"));
    return {
      bound: i.root.firstChild === i.nodes[1],
      sibling: i.nodes[1].nextSibling === i.nodes[2],
      observed: i.resizeObserver.targets.has(i.nodes[1]),
      delivered: window.delivered,
      stack: i.stack.length,
    };
  });
  expect(result).toEqual({
    bound: true,
    sibling: true,
    observed: true,
    delivered: [["focus", "1"]],
    stack: 1,
  });
});

test("compiled binary path replacement preserves the target operand", async ({
  page,
}) => {
  await runCompiledFixture(page, "path_self");
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    return {
      parent: i.root.firstChild === i.nodes[1],
      child: i.nodes[1].firstChild === i.nodes[2],
      stack: i.stack.length,
    };
  });
  expect(result).toEqual({ parent: true, child: true, stack: 1 });
});

test("compiled binary replacement moves a live subtree without retiring its descendants", async ({
  page,
}) => {
  await runCompiledFixture(page, "move_descendant");
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    window.delivered = [];
    i.nodes[2].dispatchEvent(new Event("focus"));
    return {
      old: i.nodes[1] === undefined,
      child:
        i.root.firstChild === i.nodes[2] &&
        i.nodes[2].firstChild === i.nodes[3],
      observed: i.resizeObserver.targets.has(i.nodes[2]),
      delivered: window.delivered,
      stack: i.stack.length,
    };
  });
  expect(result).toEqual({
    old: true,
    child: true,
    observed: true,
    delivered: [["focus", "2"]],
    stack: 1,
  });
});

test("compiled binary removal releases aliases, duplicate delegation and observation", async ({
  page,
}) => {
  await runCompiledFixture(page, "aliases_and_observers");
  const result = await page.evaluate(() => {
    const i = window.interpreter,
      old = i.resizeObserver.unobserved[0];
    i.resizeObserver.deliver([{ target: old }]);
    i.intersectionObserver.deliver([{ target: old }]);
    old.dispatchEvent(new Event("focus"));
    return {
      released: [1, 2, 3].every((id) => i.nodes[id] === undefined),
      globals: Object.keys(i.global),
      resize: i.resizeObserver.targets.size,
      visible: i.intersectionObserver.targets.size,
      delivered: window.delivered,
      stack: i.stack.length,
    };
  });
  expect(result).toEqual({
    released: true,
    globals: [],
    resize: 0,
    visible: 0,
    delivered: [],
    stack: 1,
  });
});

test("compiled binary removal protects a rebound slot and its current listeners", async ({
  page,
}) => {
  await runCompiledFixture(page, "reused_slot");
  const result = await page.evaluate(() => {
    const i = window.interpreter,
      fresh = i.nodes[2],
      old = i.resizeObserver.unobserved[0];
    window.delivered = [];
    old.dispatchEvent(new Event("focus"));
    fresh.dispatchEvent(new Event("focus"));
    return {
      rebound: i.root.firstChild === fresh && i.nodes[1] === undefined,
      count: i.global.click.active,
      observed:
        i.resizeObserver.targets.size === 1 &&
        i.resizeObserver.targets.has(fresh),
      oldRoute: old.getAttribute("data-dioxus-id"),
      delivered: window.delivered,
    };
  });
  expect(result).toEqual({
    rebound: true,
    count: 1,
    observed: true,
    oldRoute: null,
    delivered: [["focus", "2"]],
  });
});

test("compiled binary listener removal preserves remaining direct/delegated registrations", async ({
  page,
}) => {
  await runCompiledFixture(page, "listener_removal");
  const result = await page.evaluate(() => {
    const i = window.interpreter,
      node = i.nodes[1];
    window.delivered = [];
    node.dispatchEvent(new Event("focus"));
    node.dispatchEvent(new Event("resize"));
    return {
      delivered: window.delivered,
      route: node.getAttribute("data-dioxus-id"),
      count: i.global.click.active,
      resize: i.resizeObserver.targets.size,
      listening: node.listening,
    };
  });
  expect(result).toEqual({
    delivered: [["focus", "1"]],
    route: "1",
    count: 1,
    resize: 0,
    listening: 2,
  });
});

test("compiled binary mounted queue discards the retired physical binding", async ({
  page,
}) => {
  await runCompiledFixture(page, "mounted_reuse");
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    return {
      ids: [...i.takeMountedIds()],
      bound: i.root.firstChild === i.nodes[1],
      drained: [...i.takeMountedIds()],
    };
  });
  expect(result).toEqual({ ids: [1], bound: true, drained: [] });
});

test("compiled binary templates remain bound while retired clones are released", async ({
  page,
}) => {
  await runCompiledFixture(page, "prototype_clone");
  const result = await page.evaluate(() => {
    const i = window.interpreter;
    return {
      prototype: i.nodes[10].textContent,
      retired: i.nodes[11] === undefined,
      pending: i.nodes[12].textContent,
      distinct: i.nodes[10] !== i.nodes[12],
      top: i.stack.at(-1)[0] === i.nodes[12],
    };
  });
  expect(result).toEqual({
    prototype: "template",
    retired: true,
    pending: "template",
    distinct: true,
    top: true,
  });
});

test("compiled binary mounted instruction captures the latest stack binding", async ({
  page,
}) => {
  await runCompiledFixture(page, "mounted_set_id");
  const result = await page.evaluate(() => [
    ...window.interpreter.takeMountedIds(),
  ]);
  expect(result).toEqual([2]);
});
