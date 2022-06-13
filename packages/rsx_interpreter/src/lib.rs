use captuered_context::CapturedContext;
use dioxus_core::{NodeFactory, SchedulerMsg, VNode};
use dioxus_hooks::UnboundedSender;
use error::Error;
use interperter::build;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::Location;
use std::sync::{RwLock, RwLockReadGuard};
use syn::parse_str;

mod attributes;
pub mod captuered_context;
mod elements;
pub mod error;
mod interperter;

lazy_static! {
    /// This a a global store of the current rsx text for each call to rsx
    // Global mutable data is genrally not great, but it allows users to not worry about passing down the text RsxContex every time they switch to hot reloading.
    pub static ref RSX_CONTEXT: RsxContext = RsxContext::new(RsxData::default());
}

// the location of the code relative to the current crate based on [std::panic::Location]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Get the resolved rsx given the origional rsx, a captured context of dynamic components, and a factory to build the resulting node
pub fn resolve_scope<'a>(
    location: CodeLocation,
    rsx: &'static str,
    captured: CapturedContext<'a>,
    factory: NodeFactory<'a>,
) -> VNode<'a> {
    let rsx_text_index = &*RSX_CONTEXT;
    // only the insert the rsx text once
    if !rsx_text_index.read().hm.contains_key(&location) {
        rsx_text_index.insert(location.clone(), rsx.to_string());
    }
    if let Some(text) = {
        let read = rsx_text_index.read();
        // clone prevents deadlock on nested rsx calls
        read.hm.get(&location).cloned()
    } {
        match interpert_rsx(factory, &text, captured) {
            Ok(vnode) => vnode,
            Err(err) => {
                rsx_text_index.report_error(err);
                factory.text(format_args!(""))
            }
        }
    } else {
        panic!("rsx: line number {:?} not found in rsx index", location);
    }
}

fn interpert_rsx<'a, 'b>(
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

/// get the code location of the code that called this function
#[track_caller]
pub fn get_line_num() -> CodeLocation {
    let location = Location::caller();
    CodeLocation {
        file: location.file().to_string(),
        line: location.line(),
        column: location.column(),
    }
}

/// A handle to the rsx context with interior mutability
#[derive(Debug)]
pub struct RsxContext {
    data: RwLock<RsxData>,
}

/// A store of the text for the rsx macro for each call to rsx
#[derive(Default)]
pub struct RsxData {
    pub hm: HashMap<CodeLocation, String>,
    pub error_handler: Option<Box<dyn ErrorHandler>>,
    pub scheduler_channel: Option<UnboundedSender<SchedulerMsg>>,
}

impl std::fmt::Debug for RsxData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RsxData").field("hm", &self.hm).finish()
    }
}

impl RsxContext {
    pub fn new(data: RsxData) -> Self {
        Self {
            data: RwLock::new(data),
        }
    }

    /// Set the text for an rsx call at some location
    pub fn insert(&self, loc: CodeLocation, text: String) {
        let mut write = self.data.write().unwrap();
        write.hm.insert(loc, text);
        if let Some(channel) = &mut write.scheduler_channel {
            channel.unbounded_send(SchedulerMsg::DirtyAll).unwrap()
        }
    }

    fn read(&self) -> RwLockReadGuard<RsxData> {
        self.data.read().unwrap()
    }

    fn report_error(&self, error: Error) {
        if let Some(handler) = &self.data.write().unwrap().error_handler {
            handler.handle_error(error)
        }
    }

    /// Set the handler for errors interperting the rsx
    pub fn set_error_handler(&self, handler: impl ErrorHandler + 'static) {
        self.data.write().unwrap().error_handler = Some(Box::new(handler));
    }

    /// Provide the scduler channel from [dioxus_code::VirtualDom::get_scheduler_channel].
    /// The channel allows the interpreter to force re-rendering of the dom when the rsx is changed.
    pub fn provide_scheduler_channel(&self, channel: UnboundedSender<SchedulerMsg>) {
        self.data.write().unwrap().scheduler_channel = Some(channel)
    }
}

/// A error handler for errors reported by the rsx interperter
pub trait ErrorHandler: Send + Sync {
    fn handle_error(&self, err: Error);
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetRsxMessage {
    pub location: CodeLocation,
    pub new_text: String,
}
