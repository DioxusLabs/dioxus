//! we should properly bubble up errors from components

use std::{error::Error as StdError, marker::PhantomData, string::ParseError};

use anyhow::{anyhow, bail};
use dioxus::prelude::*;

// todo: add these to dioxus
pub trait Reject<E: Clone>: Sized {
    fn reject_err(self, t: impl FnOnce(E) -> anyhow::Error) -> Result<Self, anyhow::Error> {
        todo!()
    }
    fn reject_because(self, t: impl Into<String>) -> Result<Self, anyhow::Error> {
        todo!()
    }

    fn reject(self) -> Result<Self, anyhow::Error> {
        todo!()
    }
}

impl<T, E: Clone> Reject<E> for &Result<T, E> {
    fn reject_err(self, t: impl FnOnce(E) -> anyhow::Error) -> Result<Self, anyhow::Error> {
        todo!()
    }
}

fn use_query_param<'a>(cx: &'a ScopeState) -> Result<&'a i32, ParseError> {
    todo!()
}

/// Call "clone" on the underlying error so it can be propogated out
pub trait CloneErr<T, E: ToOwned> {
    fn clone_err(&self) -> Result<&T, E::Owned>
    where
        Self: Sized;
}

impl<E: ToOwned, T> CloneErr<T, E> for Result<T, E> {
    fn clone_err(&self) -> Result<&T, E::Owned>
    where
        Self: Sized,
    {
        match self {
            Ok(s) => Ok(s),
            Err(e) => Err(e.to_owned()),
        }
    }
}

fn app(cx: Scope) -> Element {
    // propgates error upwards, does not give a reason, lets Dioxus figure it out
    let value = cx.use_hook(|| "123123123.123".parse::<f32>()).reject()?;

    // propgates error upwards, gives a reason
    let value = cx
        .use_hook(|| "123123123.123".parse::<f32>())
        .reject_because("Parsing float failed")?;

    let value = cx.use_hook(|| "123123123.123".parse::<f32>()).clone_err()?;

    let t = use_query_param(cx)?;

    let value = cx
        .use_hook(|| "123123123.123".parse::<f32>())
        .as_ref()
        .map_err(|_| anyhow!("Parsing float failed"))?;

    todo!()
}
