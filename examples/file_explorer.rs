//! Example: File Explorer
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.
//! We store the state entirely in a single signal, making the explorer logic fairly easy to reason about.

use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(WindowBuilder::new().with_resizable(true)))
        .launch(app)
}

fn app() -> Element {
    let mut files = use_signal(Files::new);

    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/examples/assets/fileexplorer.css")
        }
        div {
            document::Link { href: "https://fonts.googleapis.com/icon?family=Material+Icons", rel: "stylesheet" }
            header {
                i { class: "material-icons icon-menu", "menu" }
                h1 { "Files: " {files.read().current()} }
                span { }
                i { class: "material-icons", onclick: move |_| files.write().go_up(), "logout" }
            }
            main {
                for (dir_id, path) in files.read().path_names.iter().enumerate() {
                    {
                        let path_end = path.split('/').last().unwrap_or(path.as_str());
                        rsx! {
                            div { class: "folder", key: "{path}",
                                i { class: "material-icons",
                                    onclick: move |_| files.write().enter_dir(dir_id),
                                    if path_end.contains('.') {
                                        "description"
                                    } else {
                                        "folder"
                                    }
                                    p { class: "cooltip", "0 folders / 0 files" }
                                }
                                h1 { "{path_end}" }
                            }
                        }
                    }
                }
                if let Some(err) = files.read().err.as_ref() {
                    div {
                        code { "{err}" }
                        button { onclick: move |_| files.write().clear_err(), "x" }
                    }
                }
            }
        }
    }
}

/// A simple little struct to hold the file explorer state
///
/// We don't use any fancy signals or memoization here - Dioxus is so fast that even a file explorer can be done with a
/// single signal.
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
        let cur_path = self.path_stack.last().unwrap();
        let paths = match std::fs::read_dir(cur_path) {
            Ok(e) => e,
            Err(err) => {
                let err = format!("An error occurred: {err:?}");
                self.err = Some(err);
                self.path_stack.pop();
                return;
            }
        };
        let collected = paths.collect::<Vec<_>>();

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        for path in collected {
            self.path_names
                .push(path.unwrap().path().display().to_string());
        }
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
