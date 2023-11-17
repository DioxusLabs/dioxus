#![allow(clippy::all, unused)]

use std::rc::Rc;

use dioxus_core::prelude::*;

use crate::{AtomRoot, Readable, Writable};

#[derive(Clone)]
pub struct CallbackApi {
    root: Rc<AtomRoot>,
}

impl CallbackApi {
    // get the current value of the atom
    pub fn get<V>(&self, atom: impl Readable<V>) -> &V {
        todo!()
    }

    // get the current value of the atom in its RC container
    pub fn get_rc<V>(&self, atom: impl Readable<V>) -> &Rc<V> {
        todo!()
    }

    // set the current value of the atom
    pub fn set<V>(&self, atom: impl Writable<V>, value: V) {
        todo!()
    }
}

#[must_use]
pub fn use_atom_context(cx: &ScopeState) -> &CallbackApi {
    todo!()
}

macro_rules! use_callback {
    (&$cx:ident, [$($cap:ident),*],  move || $body:expr) => {
        move || {
            $(
                #[allow(unused_mut)]
                let mut $cap = $cap.to_owned();
            )*
            $cx.spawn($body);
        }
    };
}

#[macro_export]
macro_rules! to_owned {
    ($($es:ident),+) => {$(
        #[allow(unused_mut)]
        let mut $es = $es.to_owned();
    )*}
}
