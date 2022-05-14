# Building a Nest
Not a bird's nest! A nest of routes!

In this chapter we will begin to build the blog portion of our site which will
include links, nested URLs, and URL parameters. We will also explore the use
case of rendering components directly in the `Router`.

## Site Navigation
Our site visitors won't know all the available pages and blogs on our site so we
should provide a navigation bar for them.
Let's create a new `NavBar` component:
```rust,ignore
#[allow(non_snake_case)]
fn NavBar(cx: Scope) -> Element {
    cx.render(rsx! {
        nav {
            ul { }
        }
    })
}
```

Our navbar will be a list of links going between our pages. We could always use
an HTML anchor element but that would cause our page to unnecessarily reload.
Instead we want to use the `Link` component provided by Dioxus Router.

The `Link` is similar to a regular `a` tag. It takes a target (for now a path,
more on targets later) and an element. Let's add our links

```rust,ignore
#[allow(non_snake_case)]
fn NavBar(cx: Scope) -> Element {
    cx.render(rsx! {
        nav {
            ul {
                // new stuff starts here
                li {
                    Link {
                        target: NtPath(String::from("/")),
                        "Home"
                    }
                }
                li {
                    Link {
                        target: NtPath(String::from("/blog")),
                        "Blog"
                    }
                }
                // new stuff ends here
            }
        }
    })
}
```

> Using this method, the `Link` component only works for links within our
> application. To learn more about navigation targets see
> [here](./navigation-targets.md).

And finally, we add the navbar component in our app component:
```rust,ignore
fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        ..Default::default()
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            fallback: RcComponent(PageNotFound),
            NavBar { } // this is new
            Outlet { }
        }
    })
}
```
Now you should see a list of links near the top of your page. Click on one and
you should seamlessly travel between pages.

### WIP: Active Link Styling

## URL Parameters and Nested Routes
Many websites such as GitHub put parameters in their URL. For example,
`https://github.com/DioxusLabs` utilizes the text after the domain to
dynamically search and display content about an organization.

We want to store our blogs in a database and load them as needed. This'll help
prevent our app from being bloated therefor providing faster load times. We also
want our users to be able to send people a link to a specific blog post.

We could utilize a search page that loads a blog when clicked but then our users
won't be able to share our blogs easily. This is where URL parameters come in.

The path to our blog will look like `/blog/myBlogPage`. `myBlogPage` being the
URL parameter.

First, lets create component that wraps around all blog content. This allows us
to add a heading that tells the user they are on the blog
```rust,ignore
#[allow(non_snake_case)]
fn Blog(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Blog" }
        Outlet {}
    })
}
```

> Note the `Outlet { }` component. For the components of a nested route to be
> rendered, we need an equally nested outlet.

Now we'll create another index component, that'll be displayed when no blog post
is selected:
```rust,ignore
#[allow(non_snake_case)]
fn BlogList(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Choose a post" }
        ul {
            li {
                Link {
                    target: NtPath(String::from("/blog/1")),
                    "Read the first blog post"
                }
            }
            li {
                Link {
                    target: NtPath(String::from("/blog/1")),
                    "Read the second blog post"
                }
            }
        }
    })
}
```

We also need to create a component that displays an actual blog post. Within
this component we can use the `use_route` hook to gain access to our URL
parameters:
```rust,ignore
#[allow(non_snake_case)]
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
```rust,ignore
fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        // new stuff starts here
        fixed: vec![(
            String::from("blog"),
            Route {
                content: RcComponent(Blog),
                sub: Some(Segment {
                    index: RcComponent(BlogList),
                    dynamic: DrParameter {
                        name: None,
                        key: "post_id",
                        content: RcComponent(BlogPost),
                        sub: None,
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
        )],
        // new stuff ends here
        ..Default::default()
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            fallback: RcComponent(PageNotFound),
            NavBar {}
            Outlet {}
        }
    })
}
```

That's it! If you head to `/blog/foo` you should see our sample post.

### Conclusion
In this chapter we utilized Dioxus Router's Link, URL Parameter, and `use_route`
functionality to build the blog portion of our application. In the next chapter,
we will go over how navigation targets (like the one we passed to our links)
work.
