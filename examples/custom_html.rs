//! This example shows how to use a custom index.html and custom <HEAD> extensions
//! to add things like stylesheets, scripts, and third-party JS libraries.

use dioxus::prelude::*;
use dioxus_desktop::Config;

const CUSTOM_HEAD: &str = r#"<style>body { background-color: red; }</style>"#;

const CUSTOM_INDEX: &str = r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Dioxus app</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <style>body { background-color: blue; }</style>
  </head>
  <body>
    <div id="main"></div>
  </body>
</html>
        "#;

fn main() {
    Config::new()
        .with_custom_head(CUSTOM_HEAD.into())
        .launch(app);

    Config::new()
        .with_custom_index(String::from(CUSTOM_INDEX))
        .launch(app);
}

fn app(cx: Scope) -> Element {
    render! {
        div {
            h1 {"hello world!"}
        }
    }
}
