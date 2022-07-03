# Core Topics

In this chapter, we'll cover some core topics about how Dioxus works and how to best leverage the features to build a beautiful, reactive app.

At a very high level, Dioxus is simply a Rust framework for _declaring_ user interfaces and _reacting_ to changes.

1) We declare what we want our user interface to look like given a state using Rust-based logic and control flow.
2) We declare how we want our state to change when the user triggers an event.

## Declarative UI

Dioxus is a *declarative* framework. This means that instead of manually writing calls to "create element" and "set element background to red," we simply *declare* what we want the element to look like and let Dioxus handle the differences.

Let's pretend that we have a stoplight we need to control - it has a color state with red, yellow, and green as options.


Using an imperative approach, we would have to manually declare each element and then handlers for advancing the stoplight.

```rust
let container = Container::new();

let green_light = Light::new().color("green").enabled(true);
let yellow_light = Light::new().color("yellow").enabled(false);
let red_light = Light::new().color("red").enabled(false);
container.push(green_light);
container.push(yellow_light);
container.push(red_light);

container.set_onclick(move |_| {
    if red_light.enabled() {
        red_light.set_enabled(false);
        green_light.set_enabled(true);
    } else if yellow_light.enabled() {
        yellow_light.set_enabled(false);
        red_light.set_enabled(true);
    } else if green_light.enabled() {
        green_light.set_enabled(false);
        yellow_light.set_enabled(true);
    }
});
```

As the UI grows in scale, our logic to keep each element in the proper state would grow exponentially. This can become very unwieldy and lead to out-of-sync UIs that harm user experience.

Instead, with Dioxus, we *declare* what we want our UI to look like:

```rust
let mut state = use_state(&cx, || "red");

cx.render(rsx!(
    Container {
        Light { color: "red", enabled: state == "red", }
        Light { color: "yellow", enabled: state == "yellow", }
        Light { color: "green", enabled: state == "green", }

        onclick: move |_| {
            state.set(match *state {
                "green" => "yellow",
                "yellow" => "red",
                "red" => "green",
            })
        }
    }
))
```

Remember: this concept is not new! Many frameworks are declarative - with React being the most popular. Declarative frameworks tend to be much more enjoyable to work with than imperative frameworks.

Here's some reading about declaring UI in React:

- [https://stackoverflow.com/questions/33655534/difference-between-declarative-and-imperative-in-react-js](https://stackoverflow.com/questions/33655534/difference-between-declarative-and-imperative-in-react-js)

- [https://medium.com/@myung.kim287/declarative-vs-imperative-251ce99c6c44](https://medium.com/@myung.kim287/declarative-vs-imperative-251ce99c6c44)
