//! A shared pool of renderers for efficient server side rendering.
use crate::prelude::*;
use crate::Result;
use crate::{document::ServerDocument, ServeConfig};
use crate::{
    streaming::{Mount, StreamingRenderer},
    IncrementalRendererError,
};
use crate::{CachedRender, IncrementalRenderer, RenderFreshness};
use dioxus_lib::document::Document;
use dioxus_lib::prelude::*;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::{collections::HashMap, future::Future};
use std::{fmt::Write, sync::RwLock};
use std::{rc::Rc, sync::Arc};
use tokio::task::JoinHandle;
