// this handler is only provided on the desktop and liveview implementations since this
// method is not used by the web implementation
this.handler = async function (event, name, bubbles) {
  let target = event.target;
  if (target != null) {
    let preventDefaultRequests = null;
    // Some events can be triggered on text nodes, which don't have attributes
    if (target instanceof Element) {
      preventDefaultRequests = target.getAttribute(`dioxus-prevent-default`);
    }

    if (event.type === "click") {
      // todo call prevent default if it's the right type of event
      if (intercept_link_redirects) {
        let a_element = target.closest("a");
        if (a_element != null) {
          event.preventDefault();

          let elementShouldPreventDefault =
            preventDefaultRequests && preventDefaultRequests.includes(`onclick`);
          let aElementShouldPreventDefault = a_element.getAttribute(
            `dioxus-prevent-default`
          );
          let linkShouldPreventDefault =
            aElementShouldPreventDefault &&
            aElementShouldPreventDefault.includes(`onclick`);

          if (!elementShouldPreventDefault && !linkShouldPreventDefault) {
            const href = a_element.getAttribute("href");
            if (href !== "" && href !== null && href !== undefined) {
              window.ipc.postMessage(
                this.serializeIpcMessage("browser_open", { href })
              );
            }
          }
        }
      }

      // also prevent buttons from submitting
      if (target.tagName === "BUTTON" && event.type == "submit") {
        event.preventDefault();
      }
    }

    const realId = find_real_id(target);

    if (
      preventDefaultRequests &&
      preventDefaultRequests.includes(`on${event.type}`)
    ) {
      event.preventDefault();
    }

    if (event.type === "submit") {
      event.preventDefault();
    }

    let contents = await serialize_event(event);

    // TODO: this should be liveview only
    if (
      target.tagName === "INPUT" &&
      (event.type === "change" || event.type === "input")
    ) {
      const type = target.getAttribute("type");
      if (type === "file") {
        async function read_files() {
          const files = target.files;
          const file_contents = {};

          for (let i = 0; i < files.length; i++) {
            const file = files[i];

            file_contents[file.name] = Array.from(
              new Uint8Array(await file.arrayBuffer())
            );
          }
          let file_engine = {
            files: file_contents,
          };
          contents.files = file_engine;

          if (realId === null) {
            return;
          }
          const message = window.interpreter.serializeIpcMessage("user_event", {
            name: name,
            element: parseInt(realId),
            data: contents,
            bubbles,
          });
          window.ipc.postMessage(message);
        }
        read_files();
        return;
      }
    }

    if (
      target.tagName === "FORM" &&
      (event.type === "submit" || event.type === "input")
    ) {
      const formData = new FormData(target);

      for (let name of formData.keys()) {
        const fieldType = target.elements[name].type;

        switch (fieldType) {
          case "select-multiple":
            contents.values[name] = formData.getAll(name);
            break;

          // add cases for fieldTypes that can hold multiple values here
          default:
            contents.values[name] = formData.get(name);
            break;
        }
      }
    }

    if (
      target.tagName === "SELECT" &&
      event.type === "input"
    ) {
      const selectData = target.options;
      contents.values["options"] = [];
      for (let i = 0; i < selectData.length; i++) {
        let option = selectData[i];
        if (option.selected) {
          contents.values["options"].push(option.value.toString());
        }
      }
    }

    if (realId === null) {
      return;
    }
    window.ipc.postMessage(
      this.serializeIpcMessage("user_event", {
        name: name,
        element: parseInt(realId),
        data: contents,
        bubbles,
      })
    );
  }
}
