export { };

declare global {
  interface Window {
    ipc: IPC;
    interpreter: NativeInterpreter;
  }
}
type NativeInterpreter = any;

class IPC {
  ws: WebSocket;

  constructor(root: Element, addr: string) {
    // window.interpreter = new NativeInterpreter();
    window.interpreter.initialize(root);
    window.interpreter.liveview = true;
    window.interpreter.ipc = this;

    const ws = new WebSocket(addr);
    ws.binaryType = "arraybuffer";

    function ping() {
      ws.send("__ping__");
    }

    ws.onopen = () => {
      // we ping every 30 seconds to keep the websocket alive
      setInterval(ping, 30000);
      ws.send(window.interpreter.serializeIpcMessage("initialize"));
    };

    ws.onerror = (err) => {
      // todo: retry the connection
    };

    ws.onmessage = (message) => {
      const u8view = new Uint8Array(message.data);
      const binaryFrame = u8view[0] == 1;
      const messageData = message.data.slice(1);

      // The first byte tells the shim if this is a binary of text frame
      if (binaryFrame) {
        // binary frame
        window.interpreter.run_from_bytes(messageData);
      } else {
        // text frame
        let decoder = new TextDecoder("utf-8");

        // Using decode method to get string output
        let str = decoder.decode(messageData);
        // Ignore pongs

        if (str != "__pong__") {
          const event = JSON.parse(str);
          switch (event.type) {
            case "query":
              Function("Eval", `"use strict";${event.data};`)();
              break;
          }
        }
      }
    };

    this.ws = ws;
  }

  postMessage(msg: string) {
    this.ws.send(msg);
  }
}

export function main(addr: string) {
  let root = window.document.getElementById("main");

  if (root != null) {
    window.ipc = new IPC(root, addr);
  }
}
