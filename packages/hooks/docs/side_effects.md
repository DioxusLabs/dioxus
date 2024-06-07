Effects are reactive closures that run **after the component has finished rendering**. Effects are useful for things like manually updating the DOM after it is rendered with web-sys or javascript. Or reading a value from the rendered DOM.

**Effects are specifically created for side effects. If you are trying to derive state, use a [memo](#derived-state), or [resource](#derived-async-state) instead.**

If you are trying to update the DOM, you can use the [`use_effect`](https://docs.rs/dioxus/latest/dioxus/prelude/fn.use_effect.html) hook to run an effect after the component has finished rendering.

`use_effect` will subscribe to any changes in the signal values it captures effects will always run after first mount and then whenever the signal values change. If the use_effect call was skipped due to an early return, the effect will no longer activate.

```rust
# use dioxus::prelude::*;
fn MyComponent() -> Element {
    let mut count = use_signal(|| 0);

    use_effect(move || {
        // Effects are reactive like memos, and resources. If you read a value inside the effect, the effect will rerun when that value changes
        let count = count.read();

        // You can use the count value to update the DOM manually
        eval(&format!(
            r#"var c = document.getElementById("dioxus-canvas");
var ctx = c.getContext("2d");
ctx.font = "30px Arial";
ctx.fillText("{count}", 10, 50);"#
        ));
    });

    rsx! {
        button {
            // When you click the button, count will be incremented and the effect will rerun
            onclick: move |_| count += 1,
            "Increment"
        }
        canvas {
            id: "dioxus-canvas",
        }
    }
}
```

## With non-reactive dependencies

To add non-reactive dependencies, you can use the [`crate::use_reactive()`] hook.

Signals will automatically be added as dependencies, so you don't need to call this method for them.

```rust
# use dioxus::prelude::*;
# async fn sleep(delay: u32) {}

#[component]
fn Comp(count: u32) -> Element {
    // Because the memo subscribes to `count` by adding it as a dependency, the memo will rerun every time `count` changes.
    use_effect(use_reactive((&count,), |(count,)| println!("Manually manipulate the dom") ));

    todo!()
}
```

## Modifying mounted nodes

One of the most common use cases for effects is modifying or reading something from the rendered DOM. Dioxus provides access to the DOM with the [`onmounted`](https://docs.rs/dioxus/latest/dioxus/events/fn.onmounted.html) event.

You can combine `use_effect` with `onmounted` to run an effect with access to a DOM element after all rendering is finished:

```rust
# use dioxus::prelude::*;
fn MyComponent() -> Element {
    let mut current_text = use_signal(String::new);
    let mut mounted_text_div: Signal<Option<MountedEvent>> = use_signal(|| None);
    let mut rendered_size = use_signal(String::new);

    use_effect(move || {
        // If we have mounted the text div, we can read the width of the div
        if let Some(div) = mounted_text_div() {
            // We read the current text here inside of the effect instead of the spawn so the effect subscribes to the signal
            let text = current_text();
            spawn(async move {
                let bounding_box = div.get_client_rect().await;
                rendered_size.set(format!("{text} is {bounding_box:?}"));
            });
        }
    });

    rsx! {
        input {
            // When you enter text into the input, the effect will rerun because it subscribes to the current_text signal
            oninput: move |evt| current_text.set(evt.value()),
            placeholder: "Enter text here",
            value: "{current_text}"
        }
        // When text changes, it will change the size of this div
        div {
            onmounted: move |element| {
                mounted_text_div.set(Some(element.clone()));
            },
            "{current_text}"
        }

        "{rendered_size}"
    }
}
```
