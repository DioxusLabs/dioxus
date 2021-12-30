//! Example: File Explorer
//! -------------------------
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.
//!
//! It also uses `use_ref` to maintain a model, rather than `use_state`. That way,
//! we dont need to clutter our code with `read` commands.

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch_cfg(app, |c| {
        c.with_window(|w| {
            w.with_resizable(true)
                .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(400.0, 800.0))
        })
    });
}

fn app(cx: Scope) -> Element {
    let files = use_ref(&cx, Files::new);

    cx.render(rsx!(
        h1 { "Files: " }
        h3 { "Cur dir: " [files.read().current()] }
        button { onclick: move |_| files.write().go_up(), "go up" }
        ol {
            files.read().path_names.iter().enumerate().map(|(dir_id, path)| rsx!(
                li { key: "{path}",
                    a { href: "#", onclick: move |_| files.write().enter_dir(dir_id),
                        "{path}",
                    }
                }
            ))
        }
        files.read().err.as_ref().map(|err| rsx!(
            div {
                code { "{err}" }
                button { onclick: move |_| files.write().clear_err(), "x" }
            }
        ))
    ))
}

struct Files {
    path_stack: Vec<String>,
    path_names: Vec<String>,
    err: Option<String>,
}

impl Files {
    fn new() -> Self {
        let mut files = Self {
            path_stack: vec!["./".to_string()],
            path_names: vec![],
            err: None,
        };

        files.reload_path_list();

        files
    }

    fn reload_path_list(&mut self) {
        let paths = match std::fs::read_dir(self.path_stack.last().unwrap()) {
            Ok(e) => e,
            Err(err) => {
                self.err = Some(format!("An error occured: {:?}", err));
                self.path_stack.pop();
                return;
            }
        };

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        self.path_names
            .extend(paths.map(|path| path.unwrap().path().display().to_string()));
    }

    fn go_up(&mut self) {
        if self.path_stack.len() > 1 {
            self.path_stack.pop();
        }
        self.reload_path_list();
    }

    fn enter_dir(&mut self, dir_id: usize) {
        let path = &self.path_names[dir_id];
        self.path_stack.push(path.clone());
        self.reload_path_list();
    }

    fn current(&self) -> &str {
        self.path_stack.last().unwrap()
    }
    fn clear_err(&mut self) {
        self.err = None;
    }
}
