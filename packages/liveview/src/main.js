function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.ipc = new IPC(root);
  }
}

class IPC {
  constructor(root) {
    // connect to the websocket
    window.interpreter = new Interpreter(root);

    let ws = new WebSocket(WS_ADDR);

    function ping() {
      ws.send("__ping__");
    }

    ws.onopen = () => {
      // we ping every 30 seconds to keep the websocket alive
      setInterval(ping, 30000);
      ws.send(serializeIpcMessage("initialize"));
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
