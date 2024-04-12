// Handle communication between rust and evaluating javascript

class Channel {
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

    recv(): Promise<any> {
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

export class Dioxus {
    sender: Channel;
    receiver: Channel;
    sendCallback: (data: any) => void;
    returnCallback: (data: any) => void;

    constructor(sendCallback, returnCallback) {
      this.sendCallback = sendCallback;
      this.returnCallback = returnCallback;
      this.promiseResolve = null;
      this.received = [];
    }
  
    // Receive message from Rust
    recv() {
      return new Promise((resolve, _reject) => {
        // If data already exists, resolve immediately
        let data = this.received.shift();
        if (data) {
          resolve(data);
          return;
        }
  
        // Otherwise set a resolve callback
        this.promiseResolve = resolve;
      });
    }
  
    // Send message to rust.
    send(data) {
      this.sendCallback(data);
    }
  
    // Internal rust send
    rustSend(data) {
      // If a promise is waiting for data, resolve it, and clear the resolve callback
      if (this.promiseResolve) {
        this.promiseResolve(data);
        this.promiseResolve = null;
        return;
      }
  
      // Otherwise add the data to a queue
      this.received.push(data);
    }
  }