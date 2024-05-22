(function () {
  var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  var url = protocol + "//" + window.location.host + "/_dioxus/ws";
  var poll_interval = 8080;

  var reload_upon_connect = (event) => {
    // Firefox will send a 1001 code when the connection is closed because the page is reloaded
    // Only firefox will trigger the onclose event when the page is reloaded manually: https://stackoverflow.com/questions/10965720/should-websocket-onclose-be-triggered-by-user-navigation-or-refresh
    // We should not reload the page in this case
    if (event.code === 1001) {
      return;
    }
    window.setTimeout(() => {
      var ws = new WebSocket(url);
      ws.onopen = () => window.location.reload();
      ws.onclose = reload_upon_connect;
    }, poll_interval);
  };

  var ws = new WebSocket(url);

  ws.onmessage = (ev) => {
    console.log("Received message: ", ev, ev.data);

    if (ev.data == "reload") {
      window.location.reload();
    }
  };

  ws.onclose = reload_upon_connect;
})();
