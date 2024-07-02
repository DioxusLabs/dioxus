// Handle communication between rust and evaluating javascript

export class Channel {
  pending: any[];
  waiting: ((data: any) => void)[];

  constructor() {
    this.pending = [];
    this.waiting = [];
  }

  send(data: any) {
    // If there's a waiting callback, call it
    if (this.waiting.length > 0) {
      this.waiting.shift()(data);
      return;
    }
    // Otherwise queue the data
    this.pending.push(data);
  }

  async recv(): Promise<any> {
    return new Promise((resolve, _reject) => {
      // If data already exists, resolve immediately
      if (this.pending.length > 0) {
        resolve(this.pending.shift());
        return;
      }
      // Otherwise queue the resolve callback
      this.waiting.push(resolve);
    });
  }
}

export class WeakDioxusChannel {
  inner: WeakRef<DioxusChannel>;

  constructor(channel: DioxusChannel) {
    this.inner = new WeakRef(channel);
  }

  // Send data from rust to javascript
  rustSend(data: any) {
    let channel = this.inner.deref();
    if (channel) {
      channel.rustSend(data);
    }
  }

  // Receive data sent from javascript in rust
  async rustRecv(): Promise<any> {
    let channel = this.inner.deref();
    if (channel) {
      return await channel.rustRecv();
    }
  }
}

export abstract class DioxusChannel {
  // Return a weak reference to this channel
  weak(): WeakDioxusChannel {
    return new WeakDioxusChannel(this);
  }

  // Send data from rust to javascript
  abstract rustSend(data: any): void;

  // Receive data sent from javascript in rust
  abstract rustRecv(): Promise<any>;
}

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
