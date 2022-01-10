# Component Properties

All component `properties` must implement the `Properties` trait. The `Props` macro automatically derives this trait but adds some additional functionality. In this section, we'll learn about:

- Using the props macro
- Memoization through PartialEq
- Optional fields on props
- The inline_props macro    




## The inline_props macro

Yes - *another* macro! However, this one is entirely optional.

For internal components, we provide the `inline_props` macro, which will let you embed your `Props` definition right into the function arguments of your component.

Our title card above would be transformed from:

```rust
#[derive(Props, PartialEq)]
struct TitleCardProps {
    title: String,
}

fn TitleCard(cx: Scope<TitleCardProps>) -> Element {
    cx.render(rsx!{
        h1 { "{cx.props.title}" }
    })
}   
```

to:

```rust
#[inline_props]
fn TitleCard(cx: Scope, title: String) -> Element {
    cx.render(rsx!{
        h1 { "{title}" }
    })
}   
```

Again, this macro is optional and should not be used by library authors since you have less fine-grained control over documentation and optionality.

However, it's great for quickly throwing together an app without dealing with *any* extra boilerplate.
