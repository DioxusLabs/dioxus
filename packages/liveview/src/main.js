const intercept_link_redirects = false;

function main() {
  let root = window.document.getElementById("main");
  if (root != null) {
    window.ipc = new IPC(root);
  }
}

class IPC {
  constructor(root) {
    window.interpreter = new NativeInterpreter();
    window.interpreter.initialize(root);
    window.interpreter.liveview = true;
    window.interpreter.ipc = this;
    const ws = new WebSocket(WS_ADDR);
    ws.binaryType = "arraybuffer";
    let pingInterval;

    function ping() {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send("__ping__");
      }
    }

    ws.onopen = () => {
      // we ping every 30 seconds to keep the websocket alive
      pingInterval = setInterval(ping, 30000);
    };

    ws.onerror = (err) => {
      // todo: retry the connection
    };

    ws.onclose = () => {
      clearInterval(pingInterval);
      for (const upload of this.activeFileUploads.values()) {
        upload.controller.abort(new Error("LiveView websocket closed during a file upload"));
      }
      this.activeFileUploads.clear();
      this.fileUploadQueue.length = 0;
      this.files.clear();
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
            case "file_upload":
              this.requestFile(event.data.id, event.data.token);
              break;
            case "file_upload_canceled":
              this.cancelFileUpload(event.data.token);
              break;
            case "file_release":
              this.releaseFile(event.data.id);
              break;
          }
        }
      }
    };

    this.ws = ws;
    this.files = new Map();
    this.fileIds = new WeakMap();
    this.fileUploadQueue = [];
    this.runningFileUploads = 0;
    this.activeFileUploads = new Map();
    this.nextFileId = 0;
  }

  postMessage(msg) {
    if (this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("LiveView websocket is not open");
    }
    this.ws.send(msg);
  }

  retainFile(file) {
    let id = this.fileIds.get(file);
    const retained = this.files.get(id);
    if (retained) {
      retained.references++;
    } else {
      id = this.nextFileId++;
      this.fileIds.set(file, id);
      this.files.set(id, { file, references: 1 });
    }
    return id;
  }

  releaseFile(id) {
    const retained = this.files.get(id);
    if (!retained || --retained.references > 0) return;
    this.files.delete(id);
    for (const [token, upload] of this.activeFileUploads) {
      if (upload.id === id) this.cancelFileUpload(token);
    }
  }

  requestFile(id, token) {
    const upload = { id, token, controller: new AbortController() };
    this.activeFileUploads.set(token, upload);
    this.fileUploadQueue.push(upload);
    this.flushFileUploads();
  }

  cancelFileUpload(token) {
    const upload = this.activeFileUploads.get(token);
    if (upload) {
      upload.controller.abort();
      this.activeFileUploads.delete(token);
      this.fileUploadQueue = this.fileUploadQueue.filter((queued) => queued !== upload);
    }
  }

  flushFileUploads() {
    // Bound concurrency across the connection, including reads from different events.
    while (this.runningFileUploads < 4 && this.fileUploadQueue.length > 0) {
      const upload = this.fileUploadQueue.shift();
      this.runningFileUploads++;
      this.sendFile(upload).catch((error) => {
        console.error("Failed to report LiveView file upload result", error);
      }).finally(() => {
        this.activeFileUploads.delete(upload.token);
        this.runningFileUploads--;
        this.flushFileUploads();
      });
    }
  }

  async sendFile({ id, token, controller }) {
    try {
      const file = this.files.get(id)?.file;
      if (!file) throw new Error("LiveView file handle was released");
      await this.uploadFile(token, file, controller.signal);
      controller.signal.throwIfAborted();
      // The server verifies that its upload handler actually received the contents.
      this.postMessage(JSON.stringify({ method: "file_upload_complete", params: { token } }));
    } catch (error) {
      if (!controller.signal.aborted && this.ws.readyState === WebSocket.OPEN) {
        this.postMessage(JSON.stringify({
          method: "file_upload_error", params: { token, error: String(error) },
        }));
      }
    }
  }

  async uploadFile(token, file, signal) {
    const url = new URL(this.ws.url);
    url.protocol = url.protocol === "wss:" ? "https:" : "http:";
    url.pathname = `${url.pathname.replace(/\/$/, "")}/upload/${encodeURIComponent(token)}`;
    url.hash = "";
    const contentLength = file.size.toString();
    const response = await fetch(url, {
      method: "PUT",
      credentials: "include",
      headers: {
        "Content-Type": file.type,
        "Content-Length": contentLength,
        "X-Content-Size": contentLength,
      },
      body: file,
      signal,
    });
    if (!response.ok) {
      throw new Error(`LiveView file upload failed with status ${response.status}`);
    }
  }
}

main();
