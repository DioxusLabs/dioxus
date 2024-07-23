import { Channel, DioxusChannel, WeakDioxusChannel } from "./eval";

// In dioxus desktop, eval needs to use the window object to store global state because we evaluate separate snippets of javascript in the browser
declare global {
  interface Window {
    __msg_queues: WeakDioxusChannel[];
    finalizationRegistry: FinalizationRegistry<{ id: number }>;

    getQuery(request_id: number): WeakDioxusChannel;

    createQuery(request_id: number): NativeDioxusChannel;
  }
}

// A message that can be sent to the desktop renderer about a query
class QueryParams {
  id: number;
  data: { method: "drop" | "send"; data?: any };

  constructor(id: number, method: "drop" | "send", data?: any) {
    this.id = id;
    this.data = { method, data };
  }
}

window.__msg_queues = window.__msg_queues || [];
// In dioxus desktop, eval is copy so we cannot run a drop handler. Instead, the drop handler is run after the channel is garbage collected in the javascript side
window.finalizationRegistry =
  window.finalizationRegistry ||
  new FinalizationRegistry(({ id }) => {
    // @ts-ignore - wry gives us this
    window.ipc.postMessage(
      JSON.stringify({
        method: "query",
        params: new QueryParams(id, "drop"),
      })
    );
  });

// Get a query from the global state
window.getQuery = function (request_id: number): WeakDioxusChannel {
  return window.__msg_queues[request_id];
};

// Create a new query (and insert it into the global state)
window.createQuery = function (request_id: number): NativeDioxusChannel {
  return new NativeDioxusChannel(request_id);
};

export class NativeDioxusChannel extends DioxusChannel {
  rust_to_js: Channel;
  request_id: number;

  constructor(request_id: number) {
    super();
    this.rust_to_js = new Channel();
    this.request_id = request_id;

    window.__msg_queues[request_id] = this.weak();
    window.finalizationRegistry.register(this, { id: request_id });
  }

  // Receive message from Rust
  async recv() {
    return await this.rust_to_js.recv();
  }

  // Send message to rust.
  send(data: any) {
    // @ts-ignore - wry gives us this
    window.ipc.postMessage(
      JSON.stringify({
        method: "query",
        params: new QueryParams(this.request_id, "send", data),
      })
    );
  }

  // Send data from rust to javascript
  rustSend(data: any) {
    this.rust_to_js.send(data);
  }

  // Receive data sent from javascript in rust. This is a no-op in the native interpreter because the rust code runs remotely
  async rustRecv(): Promise<any> {}
}
