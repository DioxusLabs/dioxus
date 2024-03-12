export class Dioxus {
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