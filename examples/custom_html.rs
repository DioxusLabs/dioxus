//! This example shows how to use a custom index.html and custom <HEAD> extensions
//! to add things like stylesheets, scripts, and third-party JS libraries.

use dioxus::prelude::*;
use dioxus_desktop::Config;

fn main() {
    LaunchBuilder::new(app).cfg(
        Config::new().with_custom_head("<style>body { background-color: red; }</style>".into()),
    );

    LaunchBuilder::new(app).cfg(
        Config::new().with_custom_index(
            r#"
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
        "#
            .into(),
        ),
    );
}

fn app() -> Element {
    render! {
        div { h1 { "hello world!" } }
    }
}
