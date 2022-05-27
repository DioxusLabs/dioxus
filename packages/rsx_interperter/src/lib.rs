use dioxus_core::{Component, Element, LazyNodes, Scope, VNode};
use dioxus_hooks::*;
use interperter::build;
use std::collections::HashMap;
use std::panic::Location;
use std::rc::Rc;
use std::sync::{RwLock, RwLockReadGuard};
use syn::parse_str;

mod attributes;
pub mod captuered_context;
mod elements;
mod interperter;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CodeLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

pub fn with_hot_reload(cx: Scope<Component>) -> Element {
    use_state(&cx, || {
        if cx.consume_context::<RsxTextIndex>().is_none() {
            let index = RsxTextIndex::default();
            cx.provide_context(index);
        }
    });
    cx.render(LazyNodes::new(|node_factory| {
        node_factory.component(*cx.props, (), None, "app")
    }))
}

pub fn interpert_rsx<'a, 'b>(
    factory: dioxus_core::NodeFactory<'a>,
    text: &str,
    context: captuered_context::CapturedContext<'a>,
) -> VNode<'a> {
    build(parse_str(text).unwrap(), context, &factory)
}

#[track_caller]
pub fn get_line_num() -> CodeLocation {
    let location = Location::caller();
    CodeLocation {
        file: location.file().to_string(),
        line: location.line(),
        column: location.column(),
    }
}

#[derive(Debug, Default, Clone)]
pub struct RsxTextIndex {
    hm: Rc<RwLock<HashMap<CodeLocation, String>>>,
}

impl RsxTextIndex {
    pub fn insert(&self, loc: CodeLocation, text: String) {
        self.hm.write().unwrap().insert(loc, text);
    }

    pub fn read(&self) -> RwLockReadGuard<HashMap<CodeLocation, String>> {
        self.hm.read().unwrap()
    }
}
