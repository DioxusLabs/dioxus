#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod check;
mod issues;
mod metadata;

pub use check::check_file;
pub use issues::{Issue, IssueReport};
