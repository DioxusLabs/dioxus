use dioxus_interpreter_js::binary_protocol::SLEDGEHAMMER_JS;

use std::io::Write;

const EDITS_PATH: &str = {
    #[cfg(any(target_os = "android", target_os = "windows"))]
    {
        "http://dioxus.index.html/edits"
    }
    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        "dioxus://index.html/edits"
    }
};

fn main() {
    let prevent_file_upload = r#"// Prevent file inputs from opening the file dialog on click
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
                const message = window.interpreter.serializeIpcMessage("file_diolog", { accept: target.getAttribute("accept"), directory: target.getAttribute("webkitdirectory") === "true", multiple: target.hasAttribute("multiple"), target: parseInt(target_id), bubbles: event_bubbles(event_name), event: event_name });
                window.ipc.postMessage(message);
              };
              send("change&input");
            }
            event.preventDefault();
          });
        }
      }
    }"#;
    let polling_request = format!(
        r#"// Poll for requests
    window.interpreter.wait_for_request = () => {{
      fetch(new Request("{EDITS_PATH}"))
          .then(response => {{
              response.arrayBuffer()
                  .then(bytes => {{
                      run_from_bytes(bytes);
                      window.interpreter.wait_for_request();
                  }});
          }})
    }}"#
    );
    let mut interpreter = SLEDGEHAMMER_JS
        .replace("/*POST_HANDLE_EDITS*/", prevent_file_upload)
        .replace("export", "")
        + &polling_request;
    while let Some(import_start) = interpreter.find("import") {
        let import_end = interpreter[import_start..]
            .find(|c| c == ';' || c == '\n')
            .map(|i| i + import_start)
            .unwrap_or_else(|| interpreter.len());
        interpreter.replace_range(import_start..import_end, "");
    }

    let js = format!("{interpreter}\nconst config = new InterpreterConfig(false);");

    use minify_js::*;
    let session = Session::new();
    let mut out = Vec::new();
    minify(&session, TopLevelMode::Module, js.as_bytes(), &mut out).unwrap();
    let minified = String::from_utf8(out).unwrap();
    let mut file = std::fs::File::create("src/minified.js").unwrap();
    file.write_all(minified.as_bytes()).unwrap();
}
