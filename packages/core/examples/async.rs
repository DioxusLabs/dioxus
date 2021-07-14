use dioxus_core::prelude::*;

fn main() {}

const App: FC<()> = |cx| {
    // create a new future
    let _fut = cx.use_hook(
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

const Task: FC<()> = |cx| {
    let (task, res) = cx.use_task(|| async { true });
    // task.pause();
    // task.restart();
    // task.stop();
    // task.drop();

    //

    let _s = cx.use_task(|| async { "hello world".to_string() });

    todo!()
};

fn use_mut<P, T>(_cx: Context<P>, _f: impl FnOnce() -> T) -> &mut T {
    todo!()
}
