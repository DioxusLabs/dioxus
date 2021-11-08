//! An event system that's less confusing than Traits + RC;
//! This should hopefully make it easier to port to other platforms.
//!
//! Unfortunately, it is less efficient than the original, but hopefully it's negligible.

use crate::{
    innerlude::Listener,
    innerlude::{ElementId, NodeFactory, ScopeId},
};
use bumpalo::boxed::Box as BumpBox;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt::Debug,
};
