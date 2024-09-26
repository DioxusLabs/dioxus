use std::time::{Duration, Instant};

use crate::builder::*;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{build::TargetArgs, bundler::AppBundle};
use futures_util::StreamExt;
use progress::{ProgressRx, ProgressTx};
use tokio::task::JoinHandle;

/// A bundler that takes in BuildResults and produces an AppBundle
pub struct Bundler {
    /// Ongoing combined bundling of app and server, if necessary
    pub bundle: Option<JoinHandle<Result<AppBundle>>>,
}
