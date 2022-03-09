# Dioxus Release Notes: v0.1.7 🏗

> Jan 7, 2022

> [@jkelleyrtp](https://github.com/jkelleyrtp)
> Thanks to [@mrxiaozhuox](https://github.com/mrxiaozhuox) [@JtotheThree](https://github.com/JtotheThree)  [@chris-morgan](https://github.com/chris-morgan) [@higumachan](https://github.com/higumachan)

TLDR Major features in this update:
- The `Props` macro now allows optional/default attributes
- InlineProps macro allows definition of props within a component's function arguments
- New router in the spirit of React Router
- `Attribute` Syntax for spreading arbitrary attributes into components
- Rehydration example, improved implementation, tests, and documentation
- File Drag n Drop support for Desktop
- PreventDefault attribute and method on events

TLDR Major fixes:
- Windows/GTK delayed loading bug fixed
- Windows ICE fixed
- Studio/CLI compiles properly

TLDR Community Contributions:
- Form Example
- Improved Calculator example
- Improved example running support

# Highlighted Features

## The `Props` macro now allows optional/default attributes

While the `Props` macro has always supported optional/default attributes, it is now documented! Props can be configured to work just like how [Typed-Builder](https://github.com/idanarye/rust-typed-builder) works:

```rust
#[derive(Props)]
struct CheckboxProps {
    #[props(default)]
    enabled: bool,

    #[props(default = "jane")]
    name: &'static str,

    #[props(auto_into)] // will always coerce Into<String>
    description: String,

    #[props(default, strip_option)]
    age: Option<usize>
}
```

## The inline props macro

In the spirit of improving props declaration, we've released the `inline_props` macro. This makes it faster to build components without needing to explicitly declare a props struct.

```rust
#[inline_props]
fn Checkbox(cx: Scope, enabled: bool, name: &'static str) -> Element {
    cx.render(rsx!{
        h1 { "Hello, {name}" }
        p { "Are you enabled?, {enabled}" }
    })
}
```

## New router in the spirit of React Router

We've added a new router in the spirit of [React Router](http://reactrouter.com). The React ecosystem has lots of experience and battle-tested solutions, so adopting React Router's architecture was easy for us.

Routes are declared 

```rust
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            ul {
                Link { to: "/",  li { "Go home!" } }
                Link { to: "users",  li { "List all users" } }
                Link { to: "blog", li { "Blog posts" } }
            }
            Route { to: "/", "Home" }
            Route { to: "users",
                Route { to: "/", "User list" }
                Route { to: ":name", BlogPost {} }
             }
            Route { to: "blog"
                Route { to: "/", "Blog list" }
                Route { to: ":post", BlogPost {} }
            }
            Route { to: "", "Err 404 Route Not Found" }
        }
    })
}

fn BlogPost(cx: Scope) -> Element {
    let post = dioxus::router::use_route(&cx).last_segment()?;

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}

fn User(cx: Scope) -> Element {
    let post = dioxus::router::use_route(&cx).last_segment()?;
    let bold = dioxus::router::use_route(&cx).param::<bool>("bold");

    cx.render(rsx! {
        div {
            h1 { "Reading blog post: {post}" }
            p { "example blog post" }
        }
    })
}
```

## `Attribute` Syntax for spreading arbitrary attributes into components


## Rehydration example, improved implementation, tests, and documentation


## File Drag n Drop support for Desktop


## PreventDefault attribute and method on events



# Highlighted Fixes

## Windows/GTK delayed loading bug fixed


## Windows ICE fixed


## Studio/CLI compiles properly


# Highlighted Community Contributions

## Form Example

## Improved Calculator example
