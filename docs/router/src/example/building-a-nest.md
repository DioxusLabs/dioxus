# Building a Nest
Not a bird's nest! A nest of routes!

In this chapter we will begin to build the blog portion of our site which will
include links, nested URLs, and URL parameters. We will also explore the use
case of rendering components directly in the [`Router`].

## Site Navigation
Our site visitors won't know all the available pages and blogs on our site so we
should provide a navigation bar for them.
Let's create a new `NavBar` component:
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
            ul { }
        }
    })
}
```

Our navbar will be a list of links going between our pages. We could always use
an HTML anchor element but that would cause our page to reload unnecessarily.
Instead we want to use the [`Link`] component provided by Dioxus Router.

The [`Link`] is similar to a regular `a` tag. It takes a target (for now a path,
more on other targets later) and an element. Let's add our links

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
                // new stuff starts here
                li { Link { target: InternalTarget(String::from("/")), "Home" } }
                li { Link {
                        target: "/blog", // short form
                        "Blog"
                } }
                // new stuff ends here
            }
        }
    })
}
```

> Using this method, the [`Link`] component only works for links within our
> application. To learn more about navigation targets see
> [here](./navigation-targets.md).

And finally, we add the navbar component in our app component:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn NavBar(cx: Scope) -> Element { unimplemented!() }
# fn PageNotFound(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Home as Component)
            .fallback(PageNotFound as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            NavBar { } // this is new
            Outlet { }
        }
    })
}
```
Now you should see a list of links near the top of your page. Click on one and
you should seamlessly travel between pages.

### Active Link Styling
You might want to style links differently, when their page is currently open.
To achieve this, we can tell the [`Link`] to give its internal `a` tag a class
in that case.

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
                li { Link {
                    target: InternalTarget(String::from("/")),
                    active_class: "active", // this is new
                    "Home"
                } }
                li { Link {
                    target: "/blog",
                    active_class: "active", // this is new
                    "Blog"
                } }
            }
        }
    })
}
```

> This will not be reflected in the [full example code](./full-code.md).

## URL Parameters and Nested Routes
Many websites such as GitHub put parameters in their URL. For example,
`https://github.com/DioxusLabs` utilizes the text after the domain to
dynamically search and display content about an organization.

We want to store our blogs in a database and load them as needed. This'll help
prevent our app from being bloated therefor providing faster load times. We also
want our users to be able to send people a link to a specific blog post.

We could utilize a search page that loads a blog when clicked but then our users
won't be able to share our blogs easily. This is where URL parameters come in.

The path to our blog will look like `/blog/myBlogPage`, `myBlogPage` being the
URL parameter.

First, lets create a component that wraps around all blog content. This allows
us to add a heading that tells the user they are on the blog.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn Blog(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Blog" }
        Outlet {}
    })
}
```

> Note the `Outlet { }` component. For the components of a nested route to be
> rendered, we need an equally nested outlet. For more details, see the
> [nested routes](../features/routes/nested.md) chapter of the features section.

Now we'll create another index component, that'll be displayed when no blog post
is selected:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Choose a post" }
        ul {
            li { Link {
                target: "/blog/1",
                "Read the first blog post"
            } }
            li { Link {
                target: "/blog/2",
                "Read the second blog post"
            } }
        }
    })
}
```

We also need to create a component that displays an actual blog post. Within
this component we can use the `use_route` hook to gain access to our URL
parameters:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();

    let post_id = route.parameters.get("post_id");
    let post = post_id
        .map(|id| id.to_string())
        .unwrap_or(String::from("unknown"));

    cx.render(rsx! {
        h2 { "Blog Post: {post}"}
    })
}
```

Finally, let's tell our router about those components.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Blog(cx: Scope) -> Element { unimplemented!() }
# fn BlogList(cx: Scope) -> Element { unimplemented!() }
# fn BlogPost(cx: Scope) -> Element { unimplemented!() }
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn NavBar(cx: Scope) -> Element { unimplemented!() }
# fn PageNotFound(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(Home as Component)
            // new stuff starts here
            .fixed(
                "blog",
                Route::new(Blog as Component).nested(
                    Segment::default()
                        .index(BlogList as Component)
                        .catch_all(("post_id", BlogPost as Component))
                ),
            )
            // new stuff ends here
            .fallback(PageNotFound as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            NavBar { }
            Outlet { }
        }
    })
}
```

That's it! If you head to `/blog/1` you should see our sample post.

## Conclusion
In this chapter we utilized Dioxus Router's Link, URL Parameter, and `use_route`
functionality to build the blog portion of our application. In the next chapter,
we will go over how navigation targets (like the one we passed to our links)
work.

[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
