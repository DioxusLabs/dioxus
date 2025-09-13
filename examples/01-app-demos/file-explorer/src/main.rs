//! Example: File Explorer
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.
//! We store the state entirely in a single signal, making the explorer logic fairly easy to reason about.

use std::env::current_dir;
use std::path::PathBuf;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut files = use_signal(Files::new);

    rsx! {
        Stylesheet { href: asset!("/assets/fileexplorer.css") }
        Stylesheet { href: "https://fonts.googleapis.com/icon?family=Material+Icons" }
        div {
            header {
                i { class: "material-icons icon-menu", "menu" }
                h1 { "Files: " {files.read().current()} }
                span { }
                i { class: "material-icons", onclick: move |_| files.write().go_up(), "logout" }
            }
            main {
                for (dir_id, path) in files.read().path_names.iter().enumerate() {
                    {
                        let path_end = path.components().next_back().map(|p|p.as_os_str()).unwrap_or(path.as_os_str()).to_string_lossy();
                        let path = path.display();
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
    current_path: PathBuf,
    path_names: Vec<PathBuf>,
    err: Option<String>,
}

impl Files {
    fn new() -> Self {
        let mut files = Self {
            current_path: std::path::absolute(current_dir().unwrap()).unwrap(),
            path_names: vec![],
            err: None,
        };

        files.reload_path_list();

        files
    }

    fn reload_path_list(&mut self) {
        let paths = match std::fs::read_dir(&self.current_path) {
            Ok(e) => e,
            Err(err) => {
                let err = format!("An error occurred: {err:?}");
                self.err = Some(err);
                return;
            }
        };
        let collected = paths.collect::<Vec<_>>();

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        for path in collected {
            self.path_names.push(path.unwrap().path().to_path_buf());
        }
    }

    fn go_up(&mut self) {
        self.current_path = match self.current_path.parent() {
            Some(path) => path.to_path_buf(),
            None => {
                self.err = Some("Cannot go up from the root directory".to_string());
                return;
            }
        };
        self.reload_path_list();
    }

    fn enter_dir(&mut self, dir_id: usize) {
        let path = &self.path_names[dir_id];
        self.current_path.clone_from(path);
        self.reload_path_list();
    }

    fn current(&self) -> String {
        self.current_path.display().to_string()
    }

    fn clear_err(&mut self) {
        self.err = None;
    }
}
