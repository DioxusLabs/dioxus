#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod copy_value;
pub use copy_value::*;

pub(crate) mod signal;
pub use signal::*;

mod read_only_signal;
pub use read_only_signal::*;

mod map;
pub use map::*;

mod set_compare;
pub use set_compare::*;

mod memo;
pub use memo::*;

mod global;
pub use global::*;

mod impls;

pub use generational_box::{
    AnyStorage, BorrowError, BorrowMutError, Owner, Storage, SyncStorage, UnsyncStorage,
};

mod read;
pub use read::*;

mod write;
pub use write::*;

mod props;
pub use props::*;
