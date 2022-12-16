function main() {
  let root = window.document.getElementById("main");

  if (root != null) {
    // create a new ipc
    window.ipc = new IPC(root);
    window.ipc.postMessage(serializeIpcMessage("initialize"));
  }
}

class IPC {
  constructor(root) {
    // connect to the websocket
    window.interpreter = new Interpreter(root);

    this.ws = new WebSocket(WS_ADDR);

    this.ws.onopen = () => {
      console.log("Connected to the websocket");
    };

    this.ws.onerror = (err) => {
      console.error("Error: ", err);
    };

    this.ws.onmessage = (event) => {
      console.log("Received message: ", event.data);
      let edits = JSON.parse(event.data);
      window.interpreter.handleEdits(edits);
    };
  }

  postMessage(msg) {
    this.ws.send(msg);
  }
}
