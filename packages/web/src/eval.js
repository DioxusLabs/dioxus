/*export class Evaluate {
  constructor(js) {
    this.js = js;
    this.callback = null;
    this.received = [];
  }
  // JS RECEIVING 
  send(val) {
    this.received.push(val);
  }
  // JS SENDING
  recv(callback) {
    this.recv = callback;
  }
  // RUN EVAL
  run(toRun, val) {
    let sendCallback = this.callback;
    let received = this.received;

    const dioxus = {
      val: val,
      send: sendCallback,
      recv: function (callback) {

        while (true) {
          let data = received.shift();
          if (data) {
            let continueRunning = callback(data);
            if (!continueRunning) {
              break;
            }
          }
        }

      }
    };

    return toRun(dioxus, val);
  }
}*/
// getDioxus(fn to send)
export class Dioxus {
  constructor(sendCallback) {
    this.sendCallback = sendCallback;
    this.received = [];
  }
  // Receive message from Rust
  recv() {
    console.log("TRYING RECV");
    let msg = null;
    while (true) {
      let data = this.received.shift();
      console.log(data);
      if (data) {
        msg = data;
        break;
      }
    }

    return msg;

    // I don't think js_sys allows async functions
    /*new Promise((resolve, _reject) => {
      let msg = null;
      while (true) {
        let data = this.received.shift();
        if (data) {
          msg = data;
          break;
        }
      }
      resolve(msg);
    });*/

  }
  // Send message to rust.
  send(data) {
    this.sendCallback(data);
  }
  // Internal rust send
  rustSend(data) {
    console.log("RECEIVED: "+data);
    this.received.push(data);
  }
}


/*
function example() {

  // Send a message
  dioxus.send("hello from js");

  let msg = dioxus.val;
  dioxus.send("I was ran with the value: "+msg);

  // Receive a message
  let msg = dioxus.recv();
}
*/