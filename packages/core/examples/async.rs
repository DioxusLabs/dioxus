use std::pin::Pin;

use dioxus_core::prelude::*;
use std::future::Future;

fn main() {}

const App: FC<()> = |cx| {
    // create a new future
    let mut fut = cx.use_hook(
        || {
            //
            async { loop {} }
            // Box::pin(async { loop {} }) as Pin<Box<dyn Future<Output = ()>>>
        },
        |f| f,
        |_| {},
    );
    // let g = unsafe { Pin::new_unchecked(fut) };

    // cx.submit_task(fut);

    todo!()
};
