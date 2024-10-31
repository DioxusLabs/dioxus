use crate::{Error, Result};
use dioxus_lib::prelude::dioxus_core::LaunchConfig;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Clone, Default)]
pub(crate) struct IndexHtml {
    pub(crate) head_before_title: String,
    pub(crate) head_after_title: String,
    pub(crate) title: String,
    pub(crate) close_head: String,
    pub(crate) post_main: String,
    pub(crate) after_closing_body_tag: String,
}

/// Get the path to the public assets directory to serve static files from
pub(crate) fn public_path() -> PathBuf {
    // The CLI always bundles static assets into the exe/public directory
    std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .unwrap()
        .join("public")
}

fn load_index_path(path: PathBuf) -> Result<String> {
    let mut file = File::open(&path).expect("No html found");

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read index.html");
    Ok(contents)
}

fn load_index_html(contents: String, root_id: &'static str) -> IndexHtml {
    let (pre_main, post_main) = contents.split_once(&format!("id=\"{root_id}\"")).unwrap_or_else(|| panic!("Failed to find id=\"{root_id}\" in index.html. The id is used to inject the application into the page."));

    let post_main = post_main.split_once('>').unwrap_or_else(|| {
        panic!("Failed to find closing > after id=\"{root_id}\" in index.html.")
    });

    let (pre_main, post_main) = (
        pre_main.to_string() + &format!("id=\"{root_id}\"") + post_main.0 + ">",
        post_main.1.to_string(),
    );

    let (head, close_head) = pre_main.split_once("</head>").unwrap_or_else(|| {
        panic!("Failed to find closing </head> tag after id=\"{root_id}\" in index.html.")
    });
    let (head, close_head) = (head.to_string(), "</head>".to_string() + close_head);

    let (post_main, after_closing_body_tag) =
        post_main.split_once("</body>").unwrap_or_else(|| {
            panic!("Failed to find closing </body> tag after id=\"{root_id}\" in index.html.")
        });

    // Strip out the head if it exists
    let mut head_before_title = String::new();
    let mut head_after_title = head;
    let mut title = String::new();
    if let Some((new_head_before_title, new_title)) = head_after_title.split_once("<title>") {
        let (new_title, new_head_after_title) = new_title
            .split_once("</title>")
            .expect("Failed to find closing </title> tag after <title> in index.html.");
        title = format!("<title>{new_title}</title>");
        head_before_title = new_head_before_title.to_string();
        head_after_title = new_head_after_title.to_string();
    }

    IndexHtml {
        head_before_title,
        head_after_title,
        title,
        close_head,
        post_main: post_main.to_string(),
        after_closing_body_tag: "</body>".to_string() + after_closing_body_tag,
    }
}
