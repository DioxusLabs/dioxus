# Conditional Rendering

Your components will often need to display different things depending on different conditions. With Dioxus, we can use Rust's normal control flow to conditional hide, show, and modify the structure of our markup.

In this chapter, you'll learn:
- How to return different Elements depending on a condition
- How to conditionally include an Element in your structure
- Common patterns like matching and bool mapping

## Conditionally returning Elements

In some components, you might want to render different markup given some condition. The typical example for conditional rendering is showing a "Log In" screen for users who aren't logged into your app. To break down this condition, we can consider two states:

- Logged in: show the app
- Logged out: show the login screen

Using the knowledge from the previous section on components, we'll start by making the app's props:

```rust
#[derive(Props, PartialEq)]
struct AppProps {
    logged_in: bool
}
```

Now that we have a "logged_in" flag accessible in our props, we can render two different screens:

```rust
fn App(cx: Scope<AppProps>) -> Element {
    if cx.props.logged_in {
        cx.render(rsx!{
            DashboardScreen {}
        })
    } else {
        cx.render(rsx!{
            LoginScreen {}
        })
    }
}
```

When the user is logged in, then this component will return the DashboardScreen. If they're not logged in, the component will render the LoginScreen.

## Using match statements

Rust provides us algebraic datatypes: enums that can contain values. Using the `match` keyword, we can execute different branches of code given a condition.

For instance, we could run a function that returns a Result:

```rust
fn App(cx: Scope)-> Element {
    match get_name() {
        Ok(name) => cx.render(rsx!( "Hello, {name}!" )),
        Err(err) => cx.render(rsx!( "Sorry, I don't know your name, because an error occurred: {err}" )),
    }
}
```

We can even match against values:
```rust
fn App(cx: Scope)-> Element {
    match get_name() {
        "jack" => cx.render(rsx!( "Hey Jack, how's Diane?" )),
        "diane" => cx.render(rsx!( "Hey Diane, how's Jack?" )),
        name => cx.render(rsx!( "Hello, {name}!" )),
    }
}
```

Do note: the `rsx!` macro returns a `Closure`, an anonymous function that has a unique type. To turn our `rsx!` into Elements, we need to call `cx.render`.

To make patterns like these less verbose, the `rsx!` macro accepts an optional first argument on which it will call `render`. Our previous component can be shortened with this alternative syntax:

```rust
fn App(cx: Scope)-> Element {
    match get_name() {
        "jack" => rsx!(cx, "Hey Jack, how's Diane?" ),
        "diane" => rsx!(cx, "Hey Diana, how's Jack?" ),
        name => rsx!(cx, "Hello, {name}!" ),
    }
}
```

Alternatively, for match statements, we can just return the builder itself and pass it into a final, single call to `cx.render`:

```rust
fn App(cx: Scope)-> Element {
    let greeting = match get_name() {
        "jack" => rsx!("Hey Jack, how's Diane?" ),
        "diane" => rsx!("Hey Diana, how's Jack?" ),
        name => rsx!("Hello, {name}!" ),
    };
    cx.render(greeting)
}
```

## Nesting RSX

By looking at other examples, you might have noticed that it's possible to include `rsx!` calls inside other `rsx!` calls. We can include anything in our `rsx!` that implements `IntoVnodeList`: a marker trait for iterators that produce Elements. `rsx!` itself implements this trait, so we can include it directly:

```rust
rsx!(
    div {
        rsx!(
            "more rsx!"
        )
    }
)
```

As you might expect, we can refactor this structure into two separate calls using variables:

```rust
let title = rsx!( "more rsx!" );

rsx!(
    div {
        title
    }
)
```

In the case of a log-in screen, we might want to display the same NavBar and Footer for both logged in and logged out users. We can model this entirely by assigning a `screen` variable to a different Element depending on a condition:


```rust
let screen = match logged_in {
    true => rsx!(DashboardScreen {}),
    false => rsx!(LoginScreen {})
};

cx.render(rsx!{
    Navbar {}
    screen,
    Footer {}
})
```


## Boolean Mapping

In the spirit of highly-functional apps, we suggest using the "boolean mapping" pattern when trying to conditionally hide/show an Element.

By default, Rust lets you convert any `boolean` into any other type by calling `and_then()`. We can exploit this functionality in components by mapping to some Element.

```rust
let show_title = true;
rsx!(
    div {
        show_title.and_then(|| rsx!{
            "This is the title"
        })
    }
)
```

We can use this pattern for many things, including options:
```rust
let user_name = Some("bob");
rsx!(
    div {
        user_name.map(|name| rsx!("Hello {name}"))
    }
)
```

## Rendering Nothing

Sometimes, you don't want your component to return anything at all. Under the hood, the `Element` type is just an alias for `Option<VNode>`, so you can simply return `None`.

This can be helpful in certain patterns where you need to perform some logical side-effects but don't want to render anything.

```rust
fn demo(cx: Scope) -> Element {
    None
}
```

## Moving Forward:

In this chapter, we learned how to render different Elements from a Component depending on a condition. This is a very powerful building block to assemble complex User Interfaces!

In the next chapter, we'll cover how to renderer lists inside your `rsx!`.

Related Reading:
- [RSX in Depth]()
