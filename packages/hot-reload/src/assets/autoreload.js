(function () {
  const POLL_INTERVAL_MIN = 250;
  const POLL_INTERVAL_MAX = 4000;
  const POLL_INTERVAL_SCALE_FACTOR = 2;

  var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  var url = protocol + "//" + window.location.host + "/_dioxus/ws";

  var reload_upon_connect = (event, poll_interval) => {
    // Firefox will send a 1001 code when the connection is closed because the page is reloaded
    // Only firefox will trigger the onclose event when the page is reloaded manually: https://stackoverflow.com/questions/10965720/should-websocket-onclose-be-triggered-by-user-navigation-or-refresh
    // We should not reload the page in this case
    if (event.code === 1001) {
      return;
    }
    window.setTimeout(() => {
      var ws = new WebSocket(url);
      ws.onopen = () => window.location.reload();
      ws.onclose = (event) => {
        reload_upon_connect(
          event,
          Math.min(
            POLL_INTERVAL_MAX,
            poll_interval * POLL_INTERVAL_SCALE_FACTOR
          )
        );
      };
    }, poll_interval);
  };

  var ws = new WebSocket(url);

  ws.onmessage = (ev) => {
    if (ev.data == "reload") {
      window.location.reload();
    }
  };

  ws.onclose = (event) => reload_upon_connect(event, POLL_INTERVAL_MIN);
})();
