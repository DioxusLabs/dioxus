export class Dioxus {
  constructor(sendCallback) {
    this.sendCallback = sendCallback;
    this.received = [];
  }
  // Receive message from Rust
  recv() {
    return new Promise((resolve, _reject) => {
      // Ever 50 ms check for new data
      let timeout = setTimeout(() => {
        let msg = null;
        while (true) {
          let data = this.received.shift();
          if (data) {
            msg = data;
            break;
          }
        }
        clearTimeout(timeout);
        resolve(msg);
      }, 50);
    });
  }
  // Send message to rust.
  send(data) {
    this.sendCallback(data);
  }
  // Internal rust send
  rustSend(data) {
    this.received.push(data);
  }
}