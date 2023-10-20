use dioxus_interpreter_js::binary_protocol::SLEDGEHAMMER_JS;
use minify_js::*;
use std::io::Write;

fn main() {
    let serialize_file_uploads = r#"if (
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
          }"#;
    let mut interpreter = SLEDGEHAMMER_JS
        .replace("/*POST_EVENT_SERIALIZATION*/", serialize_file_uploads)
        .replace("export", "");
    while let Some(import_start) = interpreter.find("import") {
        let import_end = interpreter[import_start..]
            .find(|c| c == ';' || c == '\n')
            .map(|i| i + import_start)
            .unwrap_or_else(|| interpreter.len());
        interpreter.replace_range(import_start..import_end, "");
    }

    let main_js = std::fs::read_to_string("src/main.js").unwrap();

    let js = format!("{interpreter}\n{main_js}");

    let session = Session::new();
    let mut out = Vec::new();
    minify(&session, TopLevelMode::Module, js.as_bytes(), &mut out).unwrap();
    let minified = String::from_utf8(out).unwrap();
    let mut file = std::fs::File::create("src/minified.js").unwrap();
    file.write_all(minified.as_bytes()).unwrap();
}
