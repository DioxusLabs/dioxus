#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![allow(non_snake_case)]

mod data_transfer;
pub(crate) mod file_data;

pub mod events;
pub mod geometry;
pub mod input_data;
pub mod point_interaction;

pub use crate::data_transfer::*;
pub use crate::events::*;
pub use crate::file_data::*;
pub use crate::point_interaction::*;

pub use bytes;
pub use keyboard_types::{self, Code, Key, Location, Modifiers};

pub mod traits {
    pub use crate::events::*;
    pub use crate::point_interaction::*;
}

#[cfg(feature = "serialize")]
mod transit;
#[cfg(feature = "serialize")]
pub use transit::*;
