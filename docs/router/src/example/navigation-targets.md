# Navigation Targets
In the previous chapter we learned how to create links to pages within our app.
We told them where to go using the `target` property. This property takes a
[`NavigationTarget`].

## What is a navigation target?
A [`NavigationTarget`] is similar to the `href` of an HTML anchor element.It
tells the router where to navigate to. The Dioxus Router knows three kinds of
navigation targets:
- [`Internal`]: we already saw that. It's basically an `href`, but cannot
  link to content outside our app.
- [`External`]: This works exactly like an HTML anchors `href`. In fact,
  it is just passed through. Don't use this for in-app navigation as it'll
  trigger a page reload by the browser.
- [`Named`]: this is the most interesting form of navigation target. We'll look
  at it in detail in this chapter.

## External navigation
If we need a link to an external page we can do it like this:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn GoToDioxus(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NavigationTarget::External("https://dioxuslabs.com".into()),
            "Explicit ExternalTarget target"
        }
        Link {
            target: "https://dioxuslabs.com", // short form
            "Implicit ExternalTarget target"
        }
    })
}
```

> Note that we can use a `str`, just like with [`Internal`]s. The router will
> convert a `str` to an [`External`] if the URL is absolute.

## Named navigation
When defining our routes, we can optionally give them unique static names. This
is required for a feature we call named navigation.

Up to now, when creating links we told the router the exact path to go to. With
named navigation we instead give it a name, and let it figure out the path.

This has several advantages:
- We don't have to remember absolute paths or care about what the current path
  is.
- Changing paths later on won't break internal links.
- Paths can easily be localized without affecting app logic.
- The compiler makes sure we don't have typos.

Let's try that now! First, we give our blog post route a name. We can reuse our
`BlogPost` component as a name.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Blog(cx: Scope) -> Element { unimplemented!() }
# fn BlogList(cx: Scope) -> Element { unimplemented!() }
# struct PostId;
# fn BlogPost(cx: Scope) -> Element { unimplemented!() }
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn PageNotFound(cx: Scope) -> Element { unimplemented!() }
#
struct BlogPostName;

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration::default(),
        &|| {
            Segment::content(comp(Home))
                .fixed("blog", Route::content(comp(Blog)).nested(
                    Segment::content(comp(BlogList)).catch_all(
                        ParameterRoute::content::<PostId>(comp(BlogPost))
                            .name::<BlogPostName>() // this is new
                    )
                ))
                .fallback(comp(PageNotFound))
        }
    );

    // ...
    # unimplemented!()
}
```

Now we can change the targets of the links in our `BlogList` component.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# struct PostId;
# struct BlogPostName;
# fn BlogPost(cx: Scope) -> Element { unimplemented!() }
#
fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Choose a post" }
        ul {
            li { Link {
                target: named::<BlogPostName>().parameter::<PostId>("1"),
                "Read the first blog post"
            } }
            li { Link {
                target: named::<BlogPostName>()
                    .parameter::<PostId>("1")
                    .query("query"),
                "Read the second blog post"
            } }
        }
    })
}
```

As you can see, a [`Named`] requires three fields:
1. the name to navigate to
2. a `Vec` containing all parameters that need to be inserted into the path
3. optionally a query string to use.


### The special root index name
Whether we define any names or not, the router always knows about the
[`RootIndex`] name. Navigating to it tells the router to go to `/`.

We can change the link in our `NavBar` component to take advantage of that.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn NavBar(cx: Scope) -> Element {
    cx.render(rsx! {
        nav {
            ul {
                li { Link { target: named::<RootIndex>(), "Home" } }
                li { Link { target: "/blog", "Blog" } }
            }
        }
    })
}
```


[`External`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.External
[`Internal`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Internal
[`Named`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Named
[`NavigationTarget`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html
[`RootIndex`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/prelude/struct.RootIndex.html
