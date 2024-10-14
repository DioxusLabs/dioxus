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
