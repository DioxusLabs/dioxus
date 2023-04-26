# Coroutines

Another tool in your async toolbox are coroutines. Coroutines are futures that can be manually stopped, started, paused, and resumed.

Like regular futures, code in a coroutine will run until the next `await` point before yielding. This low-level control over asynchronous tasks is quite powerful, allowing for infinitely looping tasks like WebSocket polling, background timers, and other periodic actions.

## `use_coroutine`

The `use_coroutine` hook allows you to create a coroutine. Most coroutines we write will be polling loops using async/await.

```rust
fn app(cx: Scope) -> Element {
    let ws: &UseCoroutine<()> = use_coroutine(cx, |rx| async move {
        // Connect to some sort of service
        let mut conn = connect_to_ws_server().await;

        // Wait for data on the service
        while let Some(msg) = conn.next().await {
            // handle messages
        }
    });
}
```

For many services, a simple async loop will handle the majority of use cases.

However, if we want to temporarily disable the coroutine, we can "pause" it using the `pause` method, and "resume" it using the `resume` method:

```rust
let sync: &UseCoroutine<()> = use_coroutine(cx, |rx| async move {
    // code for syncing
});

if sync.is_running() {
    cx.render(rsx!{
        button {
            onclick: move |_| sync.pause(),
            "Disable syncing"
        }
    })
} else {
    cx.render(rsx!{
        button {
            onclick: move |_| sync.resume(),
            "Enable syncing"
        }
    })
}
```

This pattern is where coroutines are extremely useful – instead of writing all the complicated logic for pausing our async tasks like we would with JavaScript promises, the Rust model allows us to just not poll our future.

## Yielding Values

To yield values from a coroutine, simply bring in a `UseState` handle and set the value whenever your coroutine completes its work.

The future must be `'static` – so any values captured by the task cannot carry any references to `cx`, such as a `UseState`.

You can use [to_owned](https://doc.rust-lang.org/std/borrow/trait.ToOwned.html#tymethod.to_owned) to create a clone of the hook handle which can be moved into the async closure.

```rust
let sync_status = use_state(cx, || Status::Launching);
let sync_task = use_coroutine(cx, |rx: UnboundedReceiver<SyncAction>| {
    let sync_status = sync_status.to_owned();
    async move {
        loop {
            delay_ms(1000).await;
            sync_status.set(Status::Working);
        }
    }
})
```

To make this a bit less verbose, Dioxus exports the `to_owned!` macro which will create a binding as shown above, which can be quite helpful when dealing with many values.

```rust
let sync_status = use_state(cx, || Status::Launching);
let load_status = use_state(cx, || Status::Launching);
let sync_task = use_coroutine(cx, |rx: UnboundedReceiver<SyncAction>| {
    to_owned![sync_status, load_status];
    async move {
        // ...
    }
})
```

## Sending Values

You might've noticed the `use_coroutine` closure takes an argument called `rx`. What is that? Well, a common pattern in complex apps is to handle a bunch of async code at once. With libraries like Redux Toolkit, managing multiple promises at once can be challenging and a common source of bugs.

With Coroutines, we can centralize our async logic. The `rx` parameter is an Channel that allows code external to the coroutine to send data *into* the coroutine. Instead of looping on an external service, we can loop on the channel itself, processing messages from within our app without needing to spawn a new future. To send data into the coroutine, we would call "send" on the handle.


```rust
use futures_util::stream::StreamExt;

enum ProfileUpdate {
    SetUsername(String),
    SetAge(i32)
}

let profile = use_coroutine(cx, |mut rx: UnboundedReciver<ProfileUpdate>| async move {
    let mut server = connect_to_server().await;

    while let Ok(msg) = rx.next().await {
        match msg {
            ProfileUpdate::SetUsername(name) => server.update_username(name).await,
            ProfileUpdate::SetAge(age) => server.update_age(age).await,
        }
    }
});


cx.render(rsx!{
    button {
        onclick: move |_| profile.send(ProfileUpdate::SetUsername("Bob".to_string())),
        "Update username"
    }
})
```


> Note: In order to use/run the `rx.next().await` statement you will need to extend the [`Stream`] trait (used by [`UnboundedReceiver`]) by adding 'futures_util' as a dependency to your project and adding the `use futures_util::stream::StreamExt;`.


For sufficiently complex apps, we could build a bunch of different useful "services" that loop on channels to update the app.

```rust
let profile = use_coroutine(cx, profile_service);
let editor = use_coroutine(cx, editor_service);
let sync = use_coroutine(cx, sync_service);

async fn profile_service(rx: UnboundedReceiver<ProfileCommand>) {
    // do stuff
}

async fn sync_service(rx: UnboundedReceiver<SyncCommand>) {
    // do stuff
}

async fn editor_service(rx: UnboundedReceiver<EditorCommand>) {
    // do stuff
}
```

We can combine coroutines with [Fermi](https://docs.rs/fermi/latest/fermi/index.html) to emulate Redux Toolkit's Thunk system with much less headache. This lets us store all of our app's state *within* a task and then simply update the "view" values stored in Atoms. It cannot be understated how powerful this technique is: we get all the perks of native Rust tasks with the optimizations and ergonomics of global state. This means your *actual* state does not need to be tied up in a system like Fermi or Redux – the only Atoms that need to exist are those that are used to drive the display/UI.

```rust
static USERNAME: Atom<String> = |_| "default".to_string();

fn app(cx: Scope) -> Element {
    let atoms = use_atom_root(cx);

    use_coroutine(cx, |rx| sync_service(rx, atoms.clone()));

    cx.render(rsx!{
        Banner {}
    })
}

fn Banner(cx: Scope) -> Element {
    let username = use_read(cx, USERNAME);

    cx.render(rsx!{
        h1 { "Welcome back, {username}" }
    })
}
```

Now, in our sync service, we can structure our state however we want. We only need to update the view values when ready.

```rust
use futures_util::stream::StreamExt;

enum SyncAction {
    SetUsername(String),
}

async fn sync_service(mut rx: UnboundedReceiver<SyncAction>, atoms: AtomRoot) {
    let username = atoms.write(USERNAME);
    let errors = atoms.write(ERRORS);

    while let Ok(msg) = rx.next().await {
        match msg {
            SyncAction::SetUsername(name) => {
                if set_name_on_server(&name).await.is_ok() {
                    username.set(name);
                } else {
                    errors.make_mut().push("SetUsernameFailed");
                }
            }
        }
    }
}
```

## Automatic injection into the Context API

Coroutine handles are automatically injected through the context API. You can use the `use_coroutine_handle` hook with the message type as a generic to fetch a handle.

```rust
fn Child(cx: Scope) -> Element {
    let sync_task = use_coroutine_handle::<SyncAction>(cx);

    sync_task.send(SyncAction::SetUsername);
}
```
