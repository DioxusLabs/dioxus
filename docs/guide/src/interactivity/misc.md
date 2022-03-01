# misc

### How do I tell Dioxus that my state changed?

Whenever you inform Dioxus that the component needs to be updated, it will "render" your component again, storing the previous and current Elements in memory. Dioxus will automatically figure out the differences between the old and the new and generate a list of edits that the renderer needs to apply to change what's on the screen. This process is called "diffing":

![Diffing](../images/diffing.png)

In React, the specifics of when a component gets re-rendered is somewhat blurry. With Dioxus, any component can mark itself as "dirty" through a method on `Context`: `needs_update`. In addition, any component can mark any _other_ component as dirty provided it knows the other component's ID with `needs_update_any`.

With these building blocks, we can craft new hooks similar to `use_state` that let us easily tell Dioxus that new information is ready to be sent to the screen.

### How do I update my state efficiently?

In general, Dioxus should be plenty fast for most use cases. However, there are some rules you should consider following to ensure your apps are quick.

- 1) **Don't call set_state _while rendering_**. This will cause Dioxus to unnecessarily re-check the component for updates or enter an infinite loop.
- 2) **Break your state apart into smaller sections.** Hooks are explicitly designed to "unshackle" your state from the typical model-view-controller paradigm, making it easy to reuse useful bits of code with a single function.
- 3) **Move local state down**. Dioxus will need to re-check child components of your app if the root component is constantly being updated. You'll get best results if rapidly-changing state does not cause major re-renders.

<!-- todo: link when the section exists
Don't worry - Dioxus is fast. But, if your app needs *extreme performance*, then take a look at the `Performance Tuning` in the `Advanced Guides` book.
-->

## The `Scope` object

Though very similar to React, Dioxus is different in a few ways. Most notably, React components will not have a `Scope` parameter in the component declaration.

Have you ever wondered how the `useState()` call works in React without a `this` object to actually store the state?

```javascript
// in React:
function Component(props) {
    // This state persists between component renders, but where does it live?
    let [state, set_state] = useState(10);
}
```

React uses global variables to store this information. However, global mutable variables must be carefully managed and are broadly discouraged in Rust programs. Because Dioxus needs to work with the rules of Rust it uses the `Scope` rather than a global state object to maintain some internal bookkeeping.

That's what the `Scope` object is: a place for the Component to store state, manage listeners, and allocate elements. Advanced users of Dioxus will want to learn how to properly leverage the `Scope` object to build robust and performant extensions for Dioxus.

```rust
fn Post(cx: Scope<PostProps>) -> Element {
    cx.render(rsx!("hello"))
}
```
