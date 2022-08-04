# Navigation Targets
In the previous chapter we learned how to create links to pages within our app.
We told them where to go using the `target` property. This property takes a
[`NavigationTarget`].

## What is a navigation target?
A [`NavigationTarget`] is similar to the `href` of an HTML anchor element.It
tells the router where to navigate to. The Dioxus Router knows three kinds of
navigation targets:
- [`NtPath`]: we already saw that. It's basically an `href`, but cannot link to
  content outside our app.
- [`NtExternal`]: This works exactly like an HTML anchors `href`. In fact, it is
  just passed through. Don't use this for in-app navigation as it'll trigger a
  page reload by the browser.
- [`NtName`]: this is the most interesting form of navigation target. We'll look
  at it in detail in this chapter.

## External navigation
If we need a link to an external page we can do it like this:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
#
#[allow(non_snake_case)]
fn GoToDioxus(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NtExternal(String::from("https://dioxuslabs.com")),
            "Explicit NtExternal target"
        }
        Link {
            target: "https://dioxuslabs.com".into(),
            "Implicit NtExternal target"
        }
    })
}
```

> Note that we can use a `str` with `.into()`, just like with [`NtPath`]
> targets. The router will convert a `str` to a [`NtExternal`] only if it starts
> with either `http://` or `https://`.

## Named navigation
When defining our routes, we can optionally give them unique static names. This
is required for a feature we call named navigation.

Up to now, when creating links we told the router the exact path to go to. With
named navigation we instead give it a name, and let it figure out the path.

This has several advantages:
- We don't hav to remember absolute paths or care about what the current path is
- changing paths later on won't break internal links
- paths can easily be localized without affecting app logic

Let's try that now! First, we give our blog post route a name.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# fn Blog(cx: Scope) -> Element { unimplemented!() }
# fn BlogList(cx: Scope) -> Element { unimplemented!() }
# fn BlogPost(cx: Scope) -> Element { unimplemented!() }
# fn Home(cx: Scope) -> Element { unimplemented!() }
#
#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(Home as Component)
            .fixed(
                "blog",
                Route::new(Blog as Component).nested(
                    Segment::default().index(BlogList as Component).parameter(
                        // notice the name at the end of the line
                        ParameterRoute::new("post_id", BlogPost as Component).name("blog_post"),
                    ),
                ),
            )
    });

    // ...
    # unimplemented!()
}
```

Now we can change the targets of the links in our `BlogList` component.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
#[allow(non_snake_case)]
fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Choose a post" }
        ul {
            li {
                Link {
                    target: NtName("blog_post", vec![("post_id", String::from("1"))], QNone),
                    "Read the first blog post"
                }
            }
            li {
                Link {
                    target: NtName("blog_post", vec![("post_id", String::from("2"))], QNone),
                    "Read the second blog post"
                }
            }
        }
    })
}
```

As you can see, [`NtName`] has three fields:
1. the name to navigate to.
2. a `Vec` containing all parameters that need to be inserted into the path
3. the query string to use. [`QNone`] means no query string


### The special root index name
Whether we define any names or not, the router always knows about the
`""` (empty string) name. Navigating to it tells the router to go to `/`.

We can change the link in our `NavBar` component to take advantage of that.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
#[allow(non_snake_case)]
fn NavBar(cx: Scope) -> Element {
    cx.render(rsx! {
        nav {
            ul {
                li {
                    Link {
                        target: NtName("", vec![], QNone),
                        "Home"
                    }
                }
                li {
                    Link {
                        target: "/blog".into(),
                        "Blog"
                    }
                }
            }
        }
    })
}
```


[`NavigationTarget`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html
[`NtExternal`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.NtExternal
[`NtName`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.NtName
[`NtPath`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.NtPath
[`QNone`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.Query.html#variant.QNone
