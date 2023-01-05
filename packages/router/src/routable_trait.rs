use std::fmt::Display;

use dioxus::core::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub trait Routable: Sized + PartialEq {
    fn from_route(route: &str) -> Option<Self> {
        todo!()
    }
    fn render<'a>(&self, cx: &'a ScopeState, route: &str) -> Element<'a> {
        todo!()
    }
}

impl<'a> Routable for &'a str {
    fn from_route(route: &str) -> Option<Self> {
        todo!()
    }

    fn render<'b>(&self, cx: &'b ScopeState, route: &str) -> Element<'b> {
        todo!()
    }
}
