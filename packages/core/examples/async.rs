use std::pin::Pin;

use dioxus_core::prelude::*;
use futures::Future;

fn main() {}

const App: FC<()> = |cx| {
    let mut fut = cx.use_hook(
        || {
            //
            Box::pin(async { loop {} }) as Pin<Box<dyn Future<Output = ()>>>
        },
        |f| f,
        |_| {},
    );

    cx.submit_task(fut);

    todo!()
};
