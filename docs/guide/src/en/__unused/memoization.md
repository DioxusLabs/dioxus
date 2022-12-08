
## Memoization

Dioxus uses Memoization for a more efficient user interface. Memoization is the process in which we check if a component actually needs to be re-rendered when its props change. If a component's properties change but they wouldn't affect the output, then we don't need to re-render the component, saving time!

For example, let's say we have a component that has two children:

```rust
fn Demo(cx: Scope) -> Element {
    // don't worry about these 2, we'll cover them later
    let name = use_state(cx, || String::from("bob"));
    let age = use_state(cx, || 21);

    cx.render(rsx!{
        Name { name: name }
        Age { age: age }
    })
}
```

If `name` changes but `age` does not, then there is no reason to re-render our `Age` component since the contents of its props did not meaningfully change.

Dioxus memoizes owned components. It uses `PartialEq` to determine if a component needs rerendering, or if it has stayed the same. This is why you must derive PartialEq!

> This means you can always rely on props with `PartialEq` or no props at all to act as barriers in your app. This can be extremely useful when building larger apps where properties frequently change. By moving our state into a global state management solution, we can achieve precise, surgical re-renders, improving the performance of our app.

Borrowed Props cannot be safely memoized. However, this is not a problem – Dioxus relies on the memoization of their parents to determine if they need to be rerendered.

