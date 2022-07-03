# UseFuture

[`use_future`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_future.html) let you run an async closure every time one of its dependencies changes. This is useful, for example, if you need to make a request based on some user input, and display the result of that request.

## Use case

The simplest use case of `use_future` is to prevent rendering until some asynchronous code has been completed. Dioxus doesn't currently have a library as sophisticated as React Query for prefetching tasks, but we can get some of the way there with `use_future`. In one of the Dioxus examples, we use `use_future` to download some search data before rendering the rest of the app:

```rust
fn app(cx: Scope) -> Element {
    // set "breeds" to the current value of the future
    let breeds = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await
    });

    let status = match breeds.value() {
        Some(Ok(val)) => "ready!",
        Some(Err(err)) => "error!",
        None => "loading!",
    }
}
```

On first run, the code inside `use_future` will be submitted to the Dioxus scheduler once the component has rendered. Since there's no data ready when the component loads the first time, its "value" will be `None`.

However, once the future is finished, the component will be re-rendered and a new screen will be displayed - Ok or Err, depending on the outcome of our fetch.



## Restarting the Future

The example we showed above will only ever run once. What happens if some value changed on the server and we need to update our future's value?

Well, the UseFuture handle provides a handy "restart" method. We can wire this up to a button or some other comparison code to get a regenerating future.

```rust
fn app(cx: Scope) -> Element {
    // set "breeds" to the current value of the future
    let dog = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await
            .unwrap()
            .json::<RandomDog>()
            .await
    });

    cx.render(match breeds.value() {
        Some(Ok(val)) => rsx!(div {
            img { src: "{val.message}"}
            button {
                onclick: move |_| dog.restart(),
                "Click to fetch a new dog"
            }
        }),
        Some(Err(err)) => rsx!("Failed to load dog"),
        None => rsx!("Loading dog image!"),
    })
}
```

## With Dependencies

We showed that UseFuture can be regenerated manually, but how can we automatically get it to update whenever some input value changes? This is where the "dependencies" tuple comes into play. We just need to add a value into our tuple argument and it'll be automatically cloned into our future when it starts.


```rust
#[inline_props]
fn RandomDog(cx: Scope, breed: String) -> Element {
    let dog = use_future(&cx, (breed,), |(breed)| async move {
        reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
            .await
            .unwrap()
            .json::<RandomDog>()
            .await
    });

    // some code as before
}
```
