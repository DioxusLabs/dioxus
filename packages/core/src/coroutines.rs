//! Coroutines are just a "futures unordered" buffer for tasks that can be submitted through the use_coroutine hook.
//!
//! The idea here is to move *coroutine* support as a layer on top of *tasks*

use futures_util::{stream::FuturesUnordered, Future};

pub struct CoroutineScheduler {
    futures: FuturesUnordered<Box<dyn Future<Output = ()>>>,
}

impl CoroutineScheduler {
    pub fn new() -> Self {
        CoroutineScheduler {
            futures: FuturesUnordered::new(),
        }
    }
}
