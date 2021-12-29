//! Examples: CSS
//! -------------
//!
//! Originally taken from:
//! - https://www.w3schools.com/html/tryit.asp?filename=tryhtml_css_internal
//!
//! Include global styles in your app!
//!
//! You can simply drop in a "style" tag and set the inner contents to your stylesheet.
//! It's slightly more manual than React, but is less magical.
//!
//! A coming update with the assets system will make it possible to include global css from child components.

use dioxus::prelude::*;

const STYLE: &str = r#"
body {background-color: powderblue;}
h1   {color: blue;}
p    {color: red;}
"#;

pub static Example: Component = |cx| {
    cx.render(rsx! {
        head { style { "{STYLE}" } }
        body {
            h1 {"This is a heading"}
            p {"This is a paragraph"}
        }
    })
};
