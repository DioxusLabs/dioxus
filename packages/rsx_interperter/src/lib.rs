use dioxus_core::{Component, Element, LazyNodes, Scope, VNode};
use dioxus_hooks::*;
use std::collections::HashMap;
use std::panic::Location;
use std::rc::Rc;
use std::sync::{RwLock, RwLockReadGuard};
use syn::parse_str;

mod attributes;
pub mod captuered_context;
mod elements;
mod interperter;

pub fn with_hot_reload(cx: Scope<Component>) -> Element {
    use_state(&cx, || {
        if cx.consume_context::<RsxTextIndex>().is_none() {
            cx.provide_context(RsxTextIndex::default());
        }
    });
    cx.render(LazyNodes::new(|node_factory| {
        node_factory.component(*cx.props, (), None, "app")
    }))
}

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
