#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![allow(unused_macros)]

::wit_bindgen::generate!({
    path: "./wit/plugin.wit",
    world: "plugin-world",
    pub_export_macro: true,
});
