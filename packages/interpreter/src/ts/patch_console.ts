
export function monkeyPatchConsole(ws: WebSocket) {
  const console = window.console;
  const log = console.log;
  const info = console.info;
  const warn = console.warn;
  const error = console.error;
  const debug = console.debug;

  // Helper function to strip console formatting placeholders
  // Console.log uses %c for styling, but we don't want to send these to the CLI
  function stripFormatting(args: any[]): any[] {
    if (args.length === 0) return args;
    
    // Check if first argument is a string with %c formatting
    if (typeof args[0] === 'string' && args[0].includes('%c')) {
      // Remove %c markers and their corresponding style arguments
      const formatString = args[0];
      const matches = formatString.match(/%c/g);
      const numFormatters = matches ? matches.length : 0;
      
      // Strip the %c markers from the format string
      const strippedFormat = formatString.replace(/%c/g, '');
      
      // Return the stripped format string and skip the style arguments
      return [strippedFormat, ...args.slice(1 + numFormatters)];
    }
    
    return args;
  }

  console.log = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "log", messages: stripFormatting(args) }
      }));
    }
    log.apply(console, args);
  };

  console.info = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "info", messages: stripFormatting(args) }
      }));
    }
    info.apply(console, args);
  };

  console.warn = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "warn", messages: stripFormatting(args) }
      }));
    }
    warn.apply(console, args);
  };

  console.error = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "error", messages: stripFormatting(args) }
      }));
    }
    error.apply(console, args);
  };

  console.debug = function (...args: any[]) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        "Log": { level: "debug", messages: stripFormatting(args) }
      }));
    }
    debug.apply(console, args);
  };
}
