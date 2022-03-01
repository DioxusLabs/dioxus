# Adding Interactivity

So far, we've learned how to describe the structure and properties of our user interfaces. Unfortunately, they're static and quite uninteresting. In this chapter, we're going to learn how to add interactivity through events, state, and tasks.

## What is state?

Every app you'll ever build has some sort of information that needs to be rendered to the screen. Dioxus is responsible for translating your desired user interface to what is rendered to the screen. *You* are responsible for providing the content.

The dynamic data in your user interface is called `State`.

When you first launch your app with `dioxus::web::launch_with_props` you'll be providing the initial state. You need to declare the initial state *before* starting the app.

```rust
fn main() {
    // declare our initial state
    let props = PostProps {
        id: Uuid::new_v4(),
        score: 10,
        comment_count: 0,
        post_time: std::time::Instant::now(),
        url: String::from("dioxuslabs.com"),
        title: String::from("Hello, world"),
        original_poster: String::from("dioxus")
    };

    // start the render loop
    dioxus::desktop::launch_with_props(Post, props);
}
```

When Dioxus renders your app, it will pass an immutable reference to `PostProps` into your `Post` component. Here, you can pass the state down into children.

```rust
fn App(cx: Scope<PostProps>) -> Element {
    cx.render(rsx!{
        Title { title: &cx.props.title }
        Score { score: &cx.props.score }
        // etc
    })
}
```

State in Dioxus follows a pattern called "one-way data-flow." As your components create new components as their children, your app's structure will eventually grow into a tree where state gets passed down from the root component into "leaves" of the tree.

You've probably seen the tree of UI components represented using a directed acyclic graph:

![image](../images/component_tree.png)

With Dioxus, your state will always flow down from parent components into child components.

## Updating State

We've talked about the data flow of state, but we haven't yet talked about how to change that state dynamically. Dioxus provides a variety of ways to change the state of your app while it's running.

For starters, we _could_ use the `update_root_props` method on the VirtualDom to provide an entirely new root state of your App. However, for most applications, you probably don't want to regenerate your entire app just to update some text or a flag.

Instead, you'll want to store state internally in your components and let *that* flow down the tree. To store state in your components, you'll use something called a `hook`. Hooks are special functions that reserve a slot of state in your component's memory and provide some functionality to update that state.

The most common hook you'll use for storing state is `use_state`. `use_state` provides a slot for some data that allows you to read and update the value without accidentally mutating it.

```rust
fn App(cx: Scope)-> Element {
    let (post, set_post) = use_state(&cx, || {
        PostData {
            id: Uuid::new_v4(),
            score: 10,
            comment_count: 0,
            post_time: std::time::Instant::now(),
            url: String::from("dioxuslabs.com"),
            title: String::from("Hello, world"),
            original_poster: String::from("dioxus")
        }
    });

    cx.render(rsx!{
        Title { title: &post.title }
        Score { score: &post.score }
        // etc
    })
}
```

Whenever we have a new post that we want to render, we can call `set_post` and provide a new value:

```rust
set_post(PostData {
    id: Uuid::new_v4(),
    score: 20,
    comment_count: 0,
    post_time: std::time::Instant::now(),
    url: String::from("google.com"),
    title: String::from("goodbye, world"),
    original_poster: String::from("google")
})
```

We'll dive deeper into how exactly these hooks work later.


## Updating State from Listeners

When responding to user-triggered events, we'll want to "listen" for an event on some element in our component.

For example, let's say we provide a button to generate a new post. Whenever the user clicks the button, they get a new post. To achieve this functionality, we'll want to attach a function to the `onclick` method of `button`. Whenever the button is clicked, our function will run, and we'll get new Post data to work with.

```rust
fn App(cx: Scope)-> Element {
    let (post, set_post) = use_state(&cx, || PostData::new());

    cx.render(rsx!{
        button {
            onclick: move |_| set_post(PostData::random())
            "Generate a random post"
        }
        Post { props: &post }
    })
}
```

We'll dive deeper into event listeners later.

## Updating State Asynchronously

We can also update our state outside of event listeners with `futures` and `coroutines`.

- `Futures` are Rust's version of promises that can execute asynchronous work by an efficient polling system. We can submit new futures to Dioxus either through `push_future` which returns a `TaskId` or with `spawn`.
- `Coroutines` are asynchronous blocks of our component that have the ability to cleanly interact with values, hooks, and other data in the component.

Since coroutines and Futures stick around between renders, the data in them must be valid for the `'static` lifetime. We must explicitly declare which values our task will rely on to avoid the `stale props` problem common in React.

We can use tasks in our components to build a tiny stopwatch that ticks every second.

> Note: The `use_future` hook will start our future immediately. The `use_coroutine` hook provides more flexibility over starting and stopping futures on the fly.

```rust
fn App(cx: Scope)-> Element {
    let (elapsed, set_elapsed) = use_state(&cx, || 0);

    use_future(&cx, (), |_| {
        // create an owned version of our value setter
        let set_elapsed = set_elapsed.clone();

        // spawn our future
        async move {
            loop {
                TimeoutFuture::from_ms(1000).await;
                set_elapsed.modify(|i| i + 1)
            }
        }
    });

    cx.render(rsx!(div { "Current stopwatch time: {sec_elapsed}" }))
}
```

Using asynchronous code can be difficult! This is just scratching the surface of what's possible. We have an entire chapter on using async properly in your Dioxus Apps.


## Moving On

This overview was a lot of information - but it doesn't tell you everything!

In the next sections we'll go over:
- `use_state` in depth
- `use_ref` and other hooks
- Handling user input

