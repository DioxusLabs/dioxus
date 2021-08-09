//! Example: File Explorer
//! -------------------------
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.

use dioxus::desktop::wry::application::dpi::LogicalSize;
use dioxus::prelude::*;
use std::fs::{self, DirEntry};

fn main() {
    env_logger::init();
    dioxus::desktop::launch(App, |c| {
        c.with_window(|w| {
            w.with_resizable(false)
                .with_inner_size(LogicalSize::new(800.0, 400.0))
        })
    })
    .unwrap();
}

static App: FC<()> = |cx| {
    let files = use_state(cx, || Files::new());

    let file_list = files.path_names.iter().enumerate().map(|(dir_id, path)| {
        rsx! (
            li { a {"{path}", onclick: move |_| files.get_mut().enter_dir(dir_id), href: "#"} }
        )
    });

    let err_disp = files.err.as_ref().map(|err| {
        rsx! {
            div {
                code {"{err}"}
                button {"x", onclick: move |_| files.get_mut().clear_err() }
            }
        }
    });

    let cur = files.current();
    cx.render(rsx! {
        div {
            h1 {"Files: "}
            h3 {"Cur dir: {cur}"}
            button { "go up", onclick: move |_| files.get_mut().go_up() }
            ol { {file_list} }
            {err_disp}
        }
    })
};

// right now, this gets cloned every time. It might be a bit better to use im_rc's collections instead
#[derive(Clone)]
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
        let paths = match fs::read_dir(cur_path) {
            Ok(e) => e,
            Err(err) => {
                let err = format!("An error occured: {:?}", err);
                self.err = Some(err);
                self.path_stack.pop();
                return;
            }
        };

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        for path in paths {
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
