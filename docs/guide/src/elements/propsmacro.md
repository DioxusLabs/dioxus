# Component Properties

All component `properties` must implement the `Properties` trait. The `Props` macro automatically derives this trait but adds some additional functionality. In this section, we'll learn about:

- Using the props macro
- Memoization through PartialEq
- Optional fields on props
- The inline_props macro    



## Using the Props Macro

All `properties` that your components take must implement the `Properties` trait. The simplest props you can use is simply `()` - or no value at all. `Scope` is generic over your component's props and actually defaults to `()`.

```rust
// this scope
Scope<()> 

// is the same as this scope
Scope
```

If we wanted to define a component with its own props, we would create a new struct and tack on the `Props` derive macro:

```rust
#[derive(Props)]
struct MyProps {
    name: String
}
```
This particular code will not compile - all `Props` must either a) borrow from their parent or b) implement `PartialEq`. Since our props do not borrow from their parent, they are `'static` and must implement PartialEq.

For an owned example:
```rust
#[derive(Props, PartialEq)]
struct MyProps {
    name: String
}
```

For a borrowed example:
```rust
#[derive(Props)]
struct MyProps<'a> {
    name: &'a str
}
```

Then, to use these props in our component, we simply swap out the generic parameter on scope.

For owned props, we just drop it in:

```rust
fn Demo(cx: Scope<MyProps>) -> Element {
    todo!()
}
```

However, for props that borrow data, we need to explicitly declare lifetimes. Rust does not know that our props and our component share the same lifetime, so must explicitly attach a lifetime in two places:

```rust
fn Demo<'a>(cx: Scope<'a, MyProps<'a>>) -> Element {
    todo!()
}
```

By putting the `'a` lifetime on Scope and our Props, we can now borrow data from our parent and pass it on to our children.


## Memoization

If you're coming from React, you might be wondering how memoization fits in. For our purpose, memoization is the process in which we check if a component actually needs to be re-rendered when its props change. If a component's properties change but they wouldn't necessarily affect the output, then we don't need to actually re-render the component.

For example, let's say we have a component that has two children:

```rust
fn Demo(cx: Scope) -> Element {
    let name = use_state(&cx, || String::from("bob"));
    let age = use_state(&cx, || 21);

    cx.render(rsx!{
        Name { name: name }
        Age { age: age }
    })
}
```

If `name` changes but `age` does not, then there is no reason to re-render our `Age` component since the contents of its props did not meaningfully change.


Dioxus implements memoization by default, which means you can always rely on props with `PartialEq` or no props at all to act as barriers in your app. This can be extremely useful when building larger apps where properties frequently change. By moving our state into a global state management solution, we can achieve precise, surgical re-renders, improving the performance of our app.


However, for components that borrow values from their parents, we cannot safely memoize them.

For example, this component borrows `&str` - and if the parent re-renders, then the actual reference to `str` will probably be different. Since the data is borrowed, we need to pass a new version down the tree.

```rust
#[derive(Props)]
struct MyProps<'a> {
    name: &'a str
}

fn Demo<'a>(cx: Scope<'a, MyProps<'a>>) -> Element {
    todo!()
}
```

TLDR: 
- if you see props with a lifetime or generics, it cannot be memoized
- memoization is done automatically through the `PartialEq` trait
- components with empty props can act as memoization barriers

## Optional Fields

Dioxus' `Props` macro is very similar to [@idanarye](https://github.com/idanarye)'s [TypedBuilder crate](https://github.com/idanarye/rust-typed-builder) and supports many of the same parameters.

For example, you can easily create optional fields by attaching the `optional` modifier to a field.

```rust
#[derive(Props, PartialEq)]
struct MyProps {
    name: String,

    #[props(optional)]
    description: Option<String>
}

fn Demo(cx: MyProps) -> Element {
    ...
}
```

Then, we can completely omit the description field when calling the component:

```rust
rsx!{
    Demo {
        name: "Thing".to_string(),
        // description is omitted
    }
}
```

The `optional` modifier is a combination of two separate modifiers: `default` and `strip_option`. The full list of modifiers includes:

- `default` - automatically add the field using its `Default` implementation
- `strip_option` - automatically wrap values at the call site in `Some`
- `optional` - combine both `default` and `strip_option`
- `into` - automatically call `into` on the value at the callsite

For more information on how tags work, check out the [TypedBuilder](https://github.com/idanarye/rust-typed-builder) crate. However, all attributes for props in Dioxus are flattened (no need for `setter` syntax) and the `optional` field is new.




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
