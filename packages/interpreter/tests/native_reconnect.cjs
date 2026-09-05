const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { test } = require("node:test");
const vm = require("node:vm");
const path = require("node:path");

const source = readFileSync(
  path.join(__dirname, "../src/js/native.js"),
  "utf8",
);

function fixture(headless = false) {
  const sockets = [],
    frames = [],
    timers = new Map(),
    applied = [];
  let nextTimer = 0;
  class Socket {
    static OPEN = 1;
    constructor(url) {
      this.url = url;
      this.readyState = 1;
      this.sent = [];
      sockets.push(this);
    }
    send(data) {
      this.sent.push(new Uint8Array(data));
    }
    close() {
      this.readyState = 3;
      this.onclose?.();
    }
    receive(data) {
      this.onmessage({ data });
    }
  }
  const context = {
    headless,
    WebSocket: Socket,
    ArrayBuffer,
    Uint8Array,
    DataView,
    RawInterpreter: class {
      run_from_bytes(bytes) {
        assert.ok(bytes instanceof ArrayBuffer);
        applied.push([...new Uint8Array(bytes)]);
      }
    },
    requestAnimationFrame: (fn) => frames.push(fn),
    setTimeout: (fn) => {
      timers.set(++nextTimer, fn);
      return nextTimer;
    },
    clearTimeout: (id) => timers.delete(id),
  };
  // Exercise the generated interpreter shipped to webviews, without a browser.
  vm.runInNewContext(
    source.replace(/export\s*\{[^}]*\};?\s*$/, "") +
      '; globalThis.interpreter = new NativeInterpreter("dioxus://", headless);',
    context,
  );
  const { interpreter } = context;
  const connect = () => {
    interpreter.waitForRequest("ws://local-test/0/key", "server-key");
    const socket = sockets.at(-1);
    socket.receive("server-key");
    return socket;
  };
  const render = () => {
    while (frames.length) frames.shift()();
  };
  return { interpreter, sockets, applied, timers, connect, render };
}

function edit(sequence, ...bytes) {
  const frame = new ArrayBuffer(8 + bytes.length);
  new DataView(frame).setBigUint64(0, BigInt(sequence), true);
  new Uint8Array(frame, 8).set(bytes);
  return frame;
}

test("resuming before rendering replays the update on the new connection", () => {
  const f = fixture();
  const old = f.connect();
  old.receive(edit(1, 42));
  const resumed = f.connect();
  resumed.receive(edit(1, 42));
  f.render();
  assert.deepEqual(f.applied, [[42]]);
  assert.equal(old.sent.length, 0);
  assert.equal(resumed.sent.length, 1);
});

test("resuming after rendering but before ACK never duplicates DOM mutations", () => {
  const f = fixture();
  const old = f.connect();
  old.receive(edit(1, 42));
  old.close();
  f.render();
  assert.deepEqual(f.applied, [[42]]);
  assert.equal(old.sent.length, 0);
  const resumed = f.connect();
  resumed.receive(edit(1, 42));
  f.render();
  resumed.receive(edit(2, 43));
  f.render();
  assert.deepEqual(f.applied, [[42], [43]]);
  assert.equal(resumed.sent.length, 2);
  assert.equal(new DataView(resumed.sent[0].buffer).getBigUint64(0, true), 1n);
});

test("stale socket events and retry timers cannot replace a resumed connection", () => {
  const f = fixture();
  const old = f.connect();
  old.close();
  const staleRetry = [...f.timers.values()][0];
  const resumed = f.connect();
  staleRetry();
  old.onclose();
  old.receive(edit(1, 99));
  resumed.receive(edit(1, 42));
  f.render();
  assert.equal(f.sockets.length, 2);
  assert.equal(f.timers.size, 0);
  assert.deepEqual(f.applied, [[42]]);
});

test("a dropped socket reconnects automatically", () => {
  const f = fixture(true);
  f.connect().close();
  [...f.timers.values()][0]();
  assert.equal(f.sockets.length, 2);
  const resumed = f.sockets.at(-1);
  resumed.receive("server-key");
  resumed.receive(edit(1, 42));
  assert.deepEqual(f.applied, [[42]]);
  assert.equal(resumed.sent.length, 1);
});

test("updates require an authenticated server connection", () => {
  const f = fixture(true);
  f.interpreter.waitForRequest("ws://local-test/0/key", "server-key");
  const socket = f.sockets[0];
  socket.receive(edit(1, 99));
  socket.receive("wrong-server-key");
  socket.receive(edit(1, 99));
  assert.deepEqual(f.applied, []);
  assert.equal(socket.readyState, 3);
});
