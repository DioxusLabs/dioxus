// Run with: node --experimental-strip-types --test patch_console.spec.ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { monkeyPatchConsole } from "./patch_console.ts";

function patched() {
  const sent: string[] = [];
  const fakeConsole: any = {};
  for (const level of ["log", "info", "warn", "error", "debug"]) {
    fakeConsole[level] = () => {};
  }
  (globalThis as any).WebSocket = { OPEN: 1 };
  (globalThis as any).window = { console: fakeConsole };
  monkeyPatchConsole({ readyState: 1, send: (s: string) => sent.push(s) } as any);
  return { fakeConsole, sent };
}

test("every console level sends only string messages", () => {
  const { fakeConsole, sent } = patched();
  for (const level of ["log", "info", "warn", "error", "debug"]) {
    fakeConsole[level]("Wee", { some: "object" }, 42, [1, 2]);
  }
  assert.equal(sent.length, 5);
  for (const raw of sent) {
    const { Log } = JSON.parse(raw);
    assert.deepEqual(Log.messages, ["Wee", '{"some":"object"}', "42", "[1,2]"]);
  }
});

test("values JSON.stringify cannot handle fall back to strings", () => {
  const { fakeConsole, sent } = patched();
  const circular: any = {};
  circular.self = circular;
  fakeConsole.debug(circular, 10n, undefined, () => {});
  const { Log } = JSON.parse(sent[0]);
  assert.equal(Log.messages.length, 4);
  for (const m of Log.messages) assert.equal(typeof m, "string");
});
