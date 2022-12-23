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

    ws.onopen = () => {
      ws.send(serializeIpcMessage("initialize"));
    };

    ws.onerror = (err) => {
      // todo: retry the connection
    };

    ws.onmessage = (event) => {
      let edits = JSON.parse(event.data);
      window.interpreter.handleEdits(edits);
    };

    this.ws = ws;
  }

  postMessage(msg) {
    this.ws.send(msg);
  }
}
