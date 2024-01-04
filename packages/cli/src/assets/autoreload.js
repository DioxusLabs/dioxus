// Dioxus-CLI
// https://github.com/DioxusLabs/dioxus/tree/master/packages/cli

(function () {
  var protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  var url = protocol + '//' + window.location.host + '/_dioxus/ws';
  var poll_interval = 8080;
  var reload_upon_connect = () => {
      window.setTimeout(
          () => {
              var ws = new WebSocket(url);
              ws.onopen = () => window.location.reload();
              ws.onclose = reload_upon_connect;
          },
          poll_interval);
  };

  var ws = new WebSocket(url);
  ws.onmessage = (ev) => {
      if (ev.data == "reload") {
          window.location.reload();
      }
  };
  ws.onclose = reload_upon_connect;
})()
