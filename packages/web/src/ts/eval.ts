import {
  DioxusChannel,
  Channel,
  WeakDioxusChannel,
} from "../../../document/src/ts/eval";

export class WebDioxusChannel extends DioxusChannel {
  js_to_rust: Channel;
  rust_to_js: Channel;
  owner: any;

  constructor(owner: any) {
    super();
    this.owner = owner;
    this.js_to_rust = new Channel();
    this.rust_to_js = new Channel();
  }

  // Return a weak reference to this channel
  weak(): WeakDioxusChannel {
    return new WeakDioxusChannel(this);
  }

  // Receive message from Rust
  async recv() {
    return await this.rust_to_js.recv();
  }

  // Send message to rust.
  send(data: any) {
    this.js_to_rust.send(data);
  }

  // Send data from rust to javascript
  rustSend(data: any) {
    this.rust_to_js.send(data);
  }

  // Receive data sent from javascript in rust
  async rustRecv(): Promise<any> {
    return await this.js_to_rust.recv();
  }
}
