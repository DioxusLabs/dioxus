use dioxus_core::{Component, Element, LazyNodes, Scope, VNode};
use dioxus_hooks::*;
use error::Error;
use interperter::build;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::Location;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use syn::parse_str;

mod attributes;
pub mod captuered_context;
mod elements;
mod error;
mod interperter;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

pub fn interpert_rsx<'a, 'b>(
    factory: dioxus_core::NodeFactory<'a>,
    text: &str,
    context: captuered_context::CapturedContext<'a>,
) -> Result<VNode<'a>, Error> {
    build(
        parse_str(text).map_err(|err| Error::ParseError(err))?,
        context,
        &factory,
    )
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

#[derive(Debug, Clone)]
pub struct RsxContext {
    data: Arc<RwLock<RsxData>>,
}

pub struct RsxData {
    pub hm: HashMap<CodeLocation, String>,
    error_handle: Box<dyn FnMut(Error)>,
}

impl std::fmt::Debug for RsxData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RsxData").field("hm", &self.hm).finish()
    }
}

impl RsxContext {
    pub fn insert(&self, loc: CodeLocation, text: String) {
        self.data.write().unwrap().hm.insert(loc, text);
    }

    pub fn read(&self) -> RwLockReadGuard<RsxData> {
        self.data.read().unwrap()
    }

    pub fn report_error(&self, error: Error) {
        (self.data.write().unwrap().error_handle)(error)
    }
}
