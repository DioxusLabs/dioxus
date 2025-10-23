#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod copy_value;
pub use copy_value::*;

pub(crate) mod signal;
pub use signal::*;

mod map;
pub use map::*;

mod map_mut;
pub use map_mut::*;

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

pub mod warnings;

mod boxed;
pub use boxed::*;

/// A macro to define extension methods for signal types that call the method with either `with` or `with_mut` depending on the mutability of self.
macro_rules! ext_methods {
    (
        $(
            $(#[$meta:meta])*
            fn $name:ident $(<$($gen:tt),*>)? (&$($self:ident)+ $(, $arg_name:ident: $arg_type:ty )* ) $(-> $ret:ty)? = $expr:expr;
        )*
    ) => {
        $(
            $(#[$meta])*
            #[track_caller]
            fn $name$(<$($gen),*>)? (& $($self)+ $(, $arg_name: $arg_type )* ) $(-> $ret)?
            {
                ext_methods!(@with $($self)+, $($arg_name),*; $expr)
            }
        )*
    };

    (@with mut $self:ident, $($arg_name:ident),*; $expr:expr) => {
        $self.with_mut(|_self| ($expr)(_self, $($arg_name),*))
    };

    (@with $self:ident, $($arg_name:ident),*; $expr:expr) => {
        $self.with(|_self| ($expr)(_self, $($arg_name),*))
    };
}

pub(crate) use ext_methods;
