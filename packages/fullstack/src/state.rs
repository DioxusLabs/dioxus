//! A shared pool of renderers for efficient server side rendering.
use crate::{document::ServerDocument, render::SsrRendererPool, ProvideServerContext, ServeConfig};
use crate::{
    streaming::{Mount, StreamingRenderer},
    DioxusServerContext,
};
use dioxus_cli_config::base_path;
use dioxus_core::{
    has_context, provide_error_boundary, DynamicNode, ErrorContext, ScopeId, SuspenseContext,
    VNode, VirtualDom,
};
use dioxus_fullstack_hooks::history::provide_fullstack_history_context;
use dioxus_fullstack_hooks::{StreamingContext, StreamingStatus};
use dioxus_fullstack_protocol::{HydrationContext, SerializedHydrationData};
use dioxus_isrg::{CachedRender, IncrementalRendererError, RenderFreshness};
use dioxus_router::ParseRouteError;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::{collections::HashMap, fmt::Write, future::Future, rc::Rc, sync::Arc, sync::RwLock};
use tokio::task::JoinHandle;

use crate::StreamingMode;
