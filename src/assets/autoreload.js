(function () {
  var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";

  var url = protocol + "//" + window.location.host + "/_dioxus/ws";

  var poll_interval = 2000;

  var ws = new WebSocket(url);

  ws.addEventListener("message", (ev) => {
    if (ev.data == "reload") {
      window.location.reload();
    }
  });
  
  ws.addEventListener("open", () => {
    ws.send("init");
  });

  ws.addEventListener("close", () => {
    alert("Dev-Server Socket Closed.");
  });

})();