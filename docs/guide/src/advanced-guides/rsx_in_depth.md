# RSX in Depth

The RSX macro makes it very easy to assemble complex UIs with a very natural Rust syntax:


```rust
rsx!(div {
    button {
        "Add todo",
        onclick: move |e| todos.write().new_todo()
    }
    ul {
        class: "todo-list"
        (todos.iter().map(|(key, todo)| rsx!(
            li { 
                class: "beautiful-todo"
                key: "f"
                h3 { "{todo.title}" }
                p { "{todo.contents}"}
            }
        )))
    }
})
```

In this section, we'll cover the `rsx!` macro in depth. If you prefer to learn through examples, the `reference` guide has plenty of examples on how to use `rsx!` effectively.



### Element structure

```rust
div {
    hidden: false,
    "some text"
    child {}
    Component {}
    {/* literal tokens that resolve to vnodes */}
}

```

Each element takes a comma-separated list of expressions to build the node. Roughly, here's how they work:

- `name: value` sets a property on this element.
- `"text"` adds a new text element
- `tag {}` adds a new child element
- `CustomTag {}` adds a new child component
- `{expr}` pastes the `expr` tokens literally. They must be `IntoIterator<T> where T: IntoVnode` to work properly

Commas are entirely optional, but might be useful to delineate between elements and attributes.

The `render` function provides an **extremely efficient** allocator for VNodes and text, so try not to use the `format!` macro in your components. Rust's default `ToString` methods pass through the global allocator, but all text in components is allocated inside a manually-managed Bump arena. To push you in the right direction, all text-based attributes take `std::fmt::Arguments` directly, so you'll want to reach for `format_args!` when the built-in `f-string` interpolation just doesn't cut it.


### Ignoring `cx.render` with `rsx!(cx, ...)`

Sometimes, writing `cx.render` is a hassle. The `rsx! macro will accept any token followed by a comma as the target to call "render" on:

```rust
cx.render(rsx!( div {} ))
// becomes
rsx!(cx, div {})
```



### Conditional Rendering

Sometimes, you might not want to render an element given a condition. The rsx! macro will accept any tokens directly contained with curly braces, provided they resolve to a type that implements `IntoIterator<VNode>`. This lets us write any Rust expression that resolves to a VNode:


```rust
rsx!({
    if enabled {
        rsx!(cx, div {"enabled"})
    } else {
        rsx!(cx, li {"disabled"})
    }
})
```
A convenient way of hiding/showing an element is returning an `Option<VNode>`. When combined with `and_then`, we can succinctly control the display state given some boolean:

```rust
rsx!({
    a.and_then(rsx!(div {"enabled"}))
})
```

It's important to note that the expression `rsx!()` is typically lazy - this expression must be _rendered_ to produce a VNode. When using match statements, we must render every arm as to avoid the `no two closures are identical` rule that Rust imposes:

```rust
// this will not compile!
match case {
    true => rsx!(div {}),
    false => rsx!(div {})
}

// the nodes must be rendered first
match case {
    true => rsx!(cx, div {}),
    false => rsx!(cx, div {})
}
```

### Lists

Again, because anything that implements `IntoIterator<VNode>` is valid, we can use lists directly in our `rsx!`:

```rust
let items = vec!["a", "b", "c"];

cx.render(rsx!{
    ul {
        {items.iter().map(|f| rsx!(li { "a" }))}
    }
})
```

Sometimes, it makes sense to render VNodes into a list:

```rust
let mut items = vec![];

for _ in 0..5 {
    items.push(rsx!(cx, li {} ))
}

rsx!(cx, {items} )
```

#### Lists and Keys

When rendering the VirtualDom to the screen, Dioxus needs to know which elements have been added and which have been removed. These changes are determined through a process called "diffing" - an old set of elements is compared to a new set of elements. If an element is removed, then it won't show up in the new elements, and Dioxus knows to remove it.

However, with lists, Dioxus does not exactly know how to determine which elements have been added or removed if the order changes or if an element is added or removed from the middle of the list.

In these cases, it is vitally important to specify a "key" alongside the element. Keys should be persistent between renders.

```rust
fn render_list(cx: Scope, items: HashMap<String, Todo>) -> DomTree {
    rsx!(cx, ul {
        {items.iter().map(|key, item| {
            li {
                key: key,
                h2 { "{todo.title}" }
                p { "{todo.contents}" }
            }
        })}
    })
}
```

There have been many guides made for keys in React, so we recommend reading up to understand their importance:

- [React guide on keys](https://reactjs.org/docs/lists-and-keys.html)
- [Importance of keys (Medium)](https://kentcdodds.com/blog/understanding-reacts-key-prop)

### Complete Reference
```rust
let text = "example";

cx.render(rsx!{
    div {
        h1 { "Example" },

        {title}

        // fstring interpolation
        "{text}"

        p {
            // Attributes
            tag: "type",

            // Anything that implements display can be an attribute
            abc: 123,
            
            enabled: true,

            // attributes also supports interpolation
            // `class` is not a restricted keyword unlike JS and ClassName
            class: "big small wide short {text}",

            class: format_args!("attributes take fmt::Arguments. {}", 99),

            tag: {"these tokens are placed directly"}

            // Children
            a { "abcder" },

            // Children with attributes
            h2 { "hello", class: "abc-123" },

            // Child components
            CustomComponent { a: 123, b: 456, key: "1" },

            // Child components with paths
            crate::components::CustomComponent { a: 123, b: 456, key: "1" },

            // Iterators
            { (0..3).map(|i| rsx!( h1 {"{:i}"} )) },

            // More rsx!, or even html!
            { rsx! { div { } } },
            { html! { <div> </div> } },

            // Matching
            // Requires rendering the nodes first.
            // rsx! is lazy, and the underlying closures cannot have the same type
            // Rendering produces the VNode type
            {match rand::gen_range::<i32>(1..3) {
                1 => rsx!(cx, h1 { "big" })
                2 => rsx!(cx, h2 { "medium" })
                _ => rsx!(cx, h3 { "small" })
            }}

            // Optionals
            {true.and_then(|f| rsx!( h1 {"Conditional Rendering"} ))}

            // Child nodes
            {cx.props.children}

            // Any expression that is `IntoVNode`
            {expr}
        }
    }
})
```
