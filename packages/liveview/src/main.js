
// Note: The following lines of this script will be removed in
// `pool.rs` in non-Debug mode before returning it to the user:

// - Lines that contain only whitespace
// - Lines that start with whitespace and `//` (i.e., comments)
// - Lines that start with whitespace and `log(` (i.e., log lines)

// TODO: Actually implement the above

function main() {
  let root = window.document.getElementById("main");
  if (root === null) {
    console.error("[Dioxus] Could not find element with ID 'main'");
  } else {
    window.ipc = new IPC(root);
  }
}

function log(msg) {
  if (DIOXUS_LOG) console.log(`[Dioxus] ${msg}`)
}

// TODO: Should there be a `logErr` variant, to always show errors, even in production?

class IPC {
  constructor(root) {
    window.interpreter = new Interpreter(root);

    const connect = () => {
      if (!this.reconnecting) {
        log("Connecting to WebSocket");
        // See `onclose` below, for why we don't log while reconnecting
      }

      this.ws = new WebSocket(DIOXUS_WS_ADDR);
      this.ws.onopen = onopen;
      this.ws.onmessage = onmessage;
      this.ws.onclose = onclose;

      // Note: `onerror` is basically useless, because it doesn't contain
      // any information about the error. `onclose` is a better way to
      // handle these scenarios. When connecting to `DIOXUS_WS_ADDR` fails,
      // `onclose` is called, as well.
    }

    const keepWsAlive = () => this.postMessage("__ping__");

    const onopen = () => {
      log("Connected to WebSocket");

      if (this.reconnecting) {
        // Remove children of `root`. Otherwise, the app will be displayed twice:
        root.innerHTML = '';
        // TODO: Do we also need to re-create `Interpreter`?
        // TODO: Do we need to use `Interpreter.Remove` instead?
        // TODO: Maybe `dioxus` itself should instruct `Interpreter`
        // to clear `root`, or at least, everything that it created?

        clearTimeout(this.reconnectDelaySetter);
        this.reconnectDelaySetter = undefined;
        this.reconnecting = false
      }
      
      this.keepWsAliveIntervalId = setInterval(keepWsAlive, 30000);
      this.postMessage(serializeIpcMessage("initialize"));
    }

    const onmessage = (event) => {
      if (event.data === "__pong__") return
      let edits = JSON.parse(event.data);
      window.interpreter.handleEdits(edits);
    }

    // TODO: Ideally, there would be a way to set up an action that is sent from the server
    // to the client after a connection is established, which is executed on disconnects
    // (e.g., to display a message that says that the server can't be reached).
    // Is there already such functionality? If not (and it would be difficult to implement),
    // this can be added via a different PR (probably by someone else).
    // I think, this could be as simple as "On disconnect, set element $selector to $rsx"

    this.reconnecting = false
    
    const onclose = (event) => {
      if (this.keepWsAliveIntervalId) {
        // Clear interval, so we don't ping the server again until we are reconnected:
        clearInterval(this.keepWsAliveIntervalId);
        this.keepWsAliveIntervalId = undefined;
      }

      if (!this.reconnecting) {
        log(`WebSocket closed – code: [${event.code}], reason: [${event.reason}]`);
        log(`Attempting to reconnect`);
        log(`Note: A WebSocket error will be logged repeatedly, until we are able to reconnect:`);

        // Browsers log un-catchable errors to the developer console, when network requests
        // fail. Those errors will potentially be logged many times until we successfully 
        // reconnect. Unfortunately, there doesn't seem to be a good way to avoid this.
        // To not add to the noise with our own log messages, we'll not log anything while
        // reconnecting.

        // TODO: I've read, that WebWorker network errors will not be logged to the developer
        // console. So moving parts of this script to a WebWorker might be a solution.
        // However, that seems to add too much complexity, to be a reasonable solution, imo.
        // With incremental re-compilation and the below delay settings the situation doesn't
        // seem to be too bad.
        
        this.reconnecting = true;

        // For the first 20 seconds, we will delay between attempts to reconnect
        // for 500 milliseconds. For the next 5 minutes, we'll delay for one second.
        // After that we'll delay for 3 seconds:
      
        this.reconnectDelay = 500;

        const increaseToOneSec = () => {
          this.reconnectDelay = 1000;
          const increaseToThreeSecs = () => this.reconnectDelay = 3000;
          this.reconnectDelaySetter = setTimeout(increaseToThreeSecs, 1000 * 60 * 5);
        }

        this.reconnectDelaySetter = setTimeout(increaseToOneSec, 1000 * 20);

        // TODO: This probably should be configurable, so that clients don't hammer the
        // server in production, when the server becomes unavailable (e.g., if the server
        // is behind a reverse proxy, that proxy would get hammered with requests until the
        // server is available again). What would be the best way to offer this configurability?
        // Probably via an argument to `dioxus_liveview::interpreter_glue`. Or this could be
        // configure via the above mentioned `disconnect action`.
      }

      setTimeout(connect, this.reconnectDelay)

      // TODO: Should 'WebSocket auto-reconnect' be optional via a Cargo feature flag?
      // It does seem to be very useful in both, production and while developing.
      // However, it is a feature that might send many requests to the server, which might
      // be unexpected. I think, having it always active and configurable (at runtime),
      // and documenting the behaviour would be the way to go. But I'm not 100% sure.
    }

    connect();
  }

  postMessage(msg) {
    if (this.ws?.readyState !== 1) {
      log("Failed to send message to server (WebSocket is not ready)");
      // See: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
      return
      // TODO: Should we return an error, so that `Interpreter` knows what's going on?
    }
    this.ws.send(msg);
  }
}
