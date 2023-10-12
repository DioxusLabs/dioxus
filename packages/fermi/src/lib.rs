#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod prelude {
    pub use crate::*;
}

mod root;

pub use atoms::*;
pub use hooks::*;
pub use root::*;

mod atoms {
    mod atom;
    mod atomfamily;
    mod atomref;
    mod selector;
    mod selectorfamily;

    pub use atom::*;
    pub use atomfamily::*;
    pub use atomref::*;
    pub use selector::*;
    pub use selectorfamily::*;
}

pub mod hooks {
    mod atom_ref;
    mod atom_root;
    mod init_atom_root;
    mod read;
    mod set;
    mod state;
    pub use atom_ref::*;
    pub use atom_root::*;
    pub use init_atom_root::*;
    pub use read::*;
    pub use set::*;
    pub use state::*;
}

/// All Atoms are `Readable` - they support reading their value.
///
/// This trait lets Dioxus abstract over Atoms, AtomFamilies, AtomRefs, and Selectors.
/// It is not very useful for your own code, but could be used to build new Atom primitives.
pub trait Readable<V> {
    fn read(&self, root: AtomRoot) -> Option<V>;
    fn init(&self) -> V;
    fn unique_id(&self) -> AtomId;
}

/// All Atoms are `Writable` - they support writing their value.
///
/// This trait lets Dioxus abstract over Atoms, AtomFamilies, AtomRefs, and Selectors.
/// This trait lets Dioxus abstract over Atoms, AtomFamilies, AtomRefs, and Selectors
pub trait Writable<V>: Readable<V> {
    fn write(&self, root: AtomRoot, value: V);
}
