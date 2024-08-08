
export function monkeyPatchConsole(ws: WebSocket) {
  const console = window.console;
  const log = console.log;
  const info = console.info;
  const warn = console.warn;
  const error = console.error;
  const debug = console.debug;

  console.log = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "log", messages: args }
      }));
    }
    log.apply(console, args);
  };

  console.info = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "info", messages: args }
      }));
    }
    info.apply(console, args);
  };

  console.warn = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "warn", messages: args }
      }));
    }
    warn.apply(console, args);
  };

  console.error = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "error", messages: args }
      }));
    }
    error.apply(console, args);
  };

  console.debug = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "debug", messages: args }
      }));
    }
    debug.apply(console, args);
  };
}
