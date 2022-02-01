# Memoization and the arena allocator

Dioxus differs slightly from other UI virtual doms in some subtle ways due to its memory allocator.

One important aspect to understand is how props are passed down from parent components to children. All "components" (custom user-made UI elements) are tightly allocated together in an arena. However, because props and hooks are generically typed, they are casted to `Any` and allocated on the heap - not in the arena with the components.

With this system, we try to be more efficient when leaving the component arena and entering the heap. By default, props are memoized between renders using COW and context. This makes props comparisons fast - done via ptr comparisons on the cow pointer. Because memoization is done by default, parent re-renders will _not_ cascade to children if the child's props did not change.

https://dmitripavlutin.com/use-react-memo-wisely/

This behavior is defined as an attribute implicit to user components. When in React land you might wrap a component with `react.memo`, Dioxus components are automatically memoized via an implicit attribute. You can manually configure this behavior on any component with "nomemo" to disable memoization.

```rust
fn test() -> DomTree {
    html! {
        <>
            <SomeComponent nomemo />
            // same as
            <SomeComponent nomemo=true />
        </>
    }
}

static TestComponent: Component = |cx| html!{<div>"Hello world"</div>};

static TestComponent: Component = |cx|{
    let g = "BLAH";
    html! {
        <div> "Hello world" </div>
    }
};

#[inline_props]
fn test_component(cx: Scope, name: String) -> Element {
    rsx!(cx, "Hello, {name}")
}
```

## Why this behavior?

"This is different than React, why differ?".

Take a component like this:

```rust
fn test(cx: Scope) -> DomTree {
    let Bundle { alpha, beta, gamma } = use_context::<SomeContext>(cx);
    html! {
        <div>
            <Component name=alpha />
            <Component name=beta />
            <Component name=gamma />
        </div>
    }
}
```

While the contents of the destructured bundle might change, not every child component will need to be re-rendered every time the context changes.
