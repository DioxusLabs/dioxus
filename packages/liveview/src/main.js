
// Note: The following lines of this script will be removed in
// `pool.rs` in non-Debug mode before returning it to the user (unless
// `InterpreterGlueBuilder::minify` is explicitly set to `false`):
// 
// - Lines that contain only whitespace
// - Lines that start with whitespace and `//` (i.e., comments)
// - Lines that start with whitespace and `log(` (i.e., log lines)

function main() {
  "use strict";
  let root = window.document.getElementById("main");
  if (root === null) {
    console.error("[Dioxus] Could not find an element with ID 'main'");
  } else {
    window.ipc = new IPC(root);
  }
}

function log(msg) {
  "use strict";
  if (DIOXUS_LOG) console.log(`[Dioxus] ${msg}`)
}

class IPC {
  constructor(root) {
    window.interpreter = new Interpreter(root);
    this.reconnecting = false;

    const connect = () => {
      if (!this.reconnecting) {
        log("Connecting to WebSocket");
        // See `onclose` below, for why we don't log while reconnecting
      }

      this.ws = new WebSocket(DIOXUS_WS_ADDR);
      this.ws.onopen = onopen;
      this.ws.onmessage = onmessage;
      this.ws.onclose = onclose;

      // Note: `onerror` is basically useless, because it doesn't contain any
      // information about the error. `onclose` is a better way to handle these
      // scenarios. When connecting to `DIOXUS_WS_ADDR` fails, `onclose` is
      // called, as well.
    }

    const keepWsAlive = () => this.postMessage("__ping__");

    const onopen = () => {
      log("Connected to WebSocket");

      if (this.reconnecting) {
        // Without the following, the app will be displayed twice:
        root.innerHTML = '';

        clearTimeout(this.reconnectDelaySetter);
        this.reconnectDelaySetter = undefined;
        this.reconnecting = false
      }
      
      this.keepWsAliveIntervalId = setInterval(keepWsAlive, 30000);

      // this.postMessage(serializeIpcMessage("initialize"));

      // XXX: The above message (which already existed when I started working
      // on this code) doesn't actually do anything. It does not serialize to a
      // message that can be deserialized to the `IpcMessage` that's defined in
      // `pool.rs`. Should this work?
    }

    const onmessage = (event) => {
      if (event.data === "__pong__") return;
      let msg = JSON.parse(event.data);
      if (msg.edits !== undefined && msg.onDisconnect !== undefined) {
        // The message we receive after (re-)connecting to the server
        this.onDisconnect = msg.onDisconnect;
        window.interpreter.handleEdits(msg.edits);
        return;
      }
      window.interpreter.handleEdits(msg);
    }

    const onclose = (event) => {
      if (this.keepWsAliveIntervalId) {
        // Clear interval, so we don't ping the server again until we are reconnected:
        clearInterval(this.keepWsAliveIntervalId);
        this.keepWsAliveIntervalId = undefined;
      }

      // If we are not already trying to reconnect, setup the reconnection machinery:
      
      if (!this.reconnecting) {
        log(`WebSocket closed – code: [${event.code}], reason: [${event.reason}]`);

        if (this.onDisconnect.length > 0) {
          log("Executing client-side disconnection actions...");
          for (const action of this.onDisconnect) {
            try {
              // if (action.type === "DangerouslyExecJs") {
              //   Function(`"use strict"; ${action.data}`)();
              //   // See `"Securing" JavaScript` on the following page for why we
              //   // add `use strict`:
              //   // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Strict_mode#securing_javascript
              // } else
              // XXX: See comment at the end of `hooks.rs` for the reason why
              // I've uncommeted `DangerouslyExecJs`
              if (action.type === "CallJsFn") {
                const fn = window[action.data];
                if (typeof fn !== "function") {
                  log(`ClientDisconnectAction Error: ${action.data} isn't a function`)
                  continue
                }
                fn()
              } else if (action.type === "SetAttribute") {
                const targets = document.querySelectorAll(action.data.selector);
                if (targets.length === 0) {
                  log(`ClientDisconnectAction Error: '${action.data.selector}' doesn't select any HTML elements`);
                  continue
                }
                for (const t of targets) {
                  t.setAttribute(action.data.name, action.data.value)
                }
              } else {
                log(`Unknown ClientDisconnectAction action: ${JSON.stringify(action)}`);
              }
            } catch (error) {
              console.error("[Error while executing `ClientDisconnectAction`]", error);
              // Continue with the next action: 
              continue;
              // XXX: I'm not 100% sure if this is the right thing to do.
              // On one hand it is unpredictable what could go wrong if we
              // continue to execute actions (e.g. actions might depend on each
              // other). On the other hand, it's also unpredictable what could
              // go wrong if we don't continue. I decided to just continue,
              // because actions normally should be isolated, and I feel it's
              // more likely that continuing results in the decired outcome.
              // If we stop, important actions that prevent data loss (e.g.
              // disabling form input elements), or that notify the visitor
              // regarding the connection loss, might not execute.
            }
          }
        }

        if (!DIOXUS_RECONNECT) {
          log("Configured to not reconnect => exit (page reload required to reconnect)")
          return
        }

        log(`Attempting to reconnect`);
        log(`Note: WebSocket errors might be logged, until we are able to reconnect`);

        // Browsers log un-catchable errors to the developer console, when
        // network requests fail. Those errors will potentially be logged many
        // times until we successfully  reconnect. Unfortunately, there doesn't
        // seem to be a good way to avoid this. To not add to the noise with
        // our own log messages, we'll not log anything while reconnecting.

        this.reconnecting = true;

        // Setting the reconnection delay according to the settings in
        // `DIOXUS_RECONNECTION_DELAYS`:

        const delays = [...DIOXUS_RECONNECT_DELAYS];

        const setReconnectDelay = () => {
          // Keep the last delay value:
          if (delays.length === 0) return;

          const [duration, delay]  = delays.shift();
          log(`Setting reconnection delay to ${delay}ms`)
          this.reconnectDelay = delay;
          this.reconnectDelaySetter = setTimeout(setReconnectDelay, duration);
        }

        setReconnectDelay();
      }

      // After a delay, try to reconnect:
      setTimeout(connect, this.reconnectDelay)
    }

    // Initial attempt to connect:
    connect();
  }

  postMessage(msg) {
    if (this.ws?.readyState !== 1) {
      log("Failed to send message to server (WebSocket is not ready)");
      // See: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
      return
      // XXX: Should we return an error, so that `Interpreter` knows what's going on?
    }
    this.ws.send(msg);
  }
}
