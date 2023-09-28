function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.ipc = new IPC(root);
  }
}

class IPC {
  constructor(root) {
    // connect to the websocket
    window.interpreter = new Interpreter(root, new InterpreterConfig(false));

    let ws = new WebSocket(WS_ADDR);

    function ping() {
      ws.send("__ping__");
    }

    ws.onopen = () => {
      // we ping every 30 seconds to keep the websocket alive
      setInterval(ping, 30000);
      ws.send(serializeIpcMessage("initialize"));

      // Send initial path and session data
      ws.send(serializeIpcMessage("window_event", {
        type: "load",
        params: {
          location: {
            path: document.location.pathname,
            search: document.location.search,
            hash: document.location.hash,
          },
          state: JSON.stringify(history.state),
          session: JSON.stringify(sessionStorage),
          depth: history.length,
        },
      }));

      // Send updates to history
      window.addEventListener("popstate", (event) => {
        ws.send(serializeIpcMessage("window_event", {
          type: "popstate",
          params: {
            location: {
              path: document.location.pathname,
              search: document.location.search,
              hash: document.location.hash,
            },
            state: JSON.stringify(event.state),
          },
        }));
      });
    };

    ws.onerror = (err) => {
      // todo: retry the connection
    };

    ws.onmessage = (message) => {
      // Ignore pongs
      if (message.data != "__pong__") {
        const event = JSON.parse(message.data);
        switch (event.type) {
          case "edits":
            let edits = event.data;
            window.interpreter.handleEdits(edits);
            break;
          case "query":
            Function("Eval", `"use strict";${event.data};`)();
            break;
        }
      }
    };

    this.ws = ws;
  }

  postMessage(msg) {
    this.ws.send(msg);
  }
}
