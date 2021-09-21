use dioxus_core::prelude::*;

fn main() {}

const App: FC<()> = |cx, props| {
    // create a new future
    let _fut = cx.use_hook(
        |_| {
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

const Task: FC<()> = |cx, props| {
    let (_task, _res) = use_task(cx, || async { true });
    // task.pause();
    // task.restart();
    // task.stop();
    // task.drop();

    //

    let _s = use_task(cx, || async { "hello world".to_string() });

    todo!()
};

fn use_mut<P, T>(_cx: Context, _f: impl FnOnce() -> T) -> &mut T {
    todo!()
}
