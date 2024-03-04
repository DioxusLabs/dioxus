// Prevent file inputs from opening the file dialog on click
  let inputs = document.querySelectorAll("input");
  for (let input of inputs) {
    if (!input.getAttribute("data-dioxus-file-listener")) {
      // prevent file inputs from opening the file dialog on click
      const type = input.getAttribute("type");
      if (type === "file") {
        input.setAttribute("data-dioxus-file-listener", true);
        input.addEventListener("click", (event) => {
          let target = event.target;
          let target_id = find_real_id(target);
          if (target_id !== null) {
            const send = (event_name) => {
              const message = window.interpreter.serializeIpcMessage("file_dialog", { accept: target.getAttribute("accept"), directory: target.getAttribute("webkitdirectory") === "true", multiple: target.hasAttribute("multiple"), target: parseInt(target_id), bubbles: event_bubbles(event_name), event: event_name });
              window.ipc.postMessage(message);
            };
            send("change&input");
          }
          event.preventDefault();
        });
      }
    }
  }
