use crate::interperter::build;
use dioxus_core::{LazyNodes, VNode};
use dioxus_rsx::CallBody;
use std::collections::HashMap;
use std::panic::Location;
use std::rc::Rc;
use std::sync::{RwLock, RwLockReadGuard};
use syn::{parse_str, Result};

mod attributes;
pub mod captuered_context;
mod elements;
mod interperter;

pub fn interpert_rsx<'a, 'b>(
    factory: dioxus_core::NodeFactory<'a>,
    text: &str,
    context: captuered_context::CapturedContext,
) -> VNode<'a> {
    panic!()
}

#[track_caller]
pub fn get_line_num() -> &'static Location<'static> {
    Location::caller()
}

#[derive(Debug, Default, Clone)]
pub struct RsxTextIndex {
    hm: Rc<RwLock<HashMap<&'static Location<'static>, String>>>,
}

impl RsxTextIndex {
    pub fn insert(&self, loc: &'static Location<'static>, text: String) {
        self.hm.write().unwrap().insert(loc, text);
    }

    pub fn read(&self) -> RwLockReadGuard<HashMap<&'static Location<'static>, String>> {
        self.hm.read().unwrap()
    }
}
