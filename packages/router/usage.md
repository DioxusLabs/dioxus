
## Usage

Using Dioxus Router is pretty simple. Add a top-level Router to your app (not necessary but good practice) and then start adding routes, specifying the "to" field:

```rust
fn app() {
    cx.render(rsx! {
        Router {
            Route { to: "/", Component {} },
            Route { to: "/blog", Blog {} },
            Route { to: "/about", Blog {} },
            Route { to: "/contact", Blog {} },
            Route { to: "/shop", Blog {} },
        }
    })
}
```

All Routes must start with a forward slash.

To have dynamic route segments, use the `:id` syntax. If concrete paths come *before* the dynamic syntax, then those will be chosen first.

```rust
cx.render(rsx! {
    Router {
        Route { to: "/", Component {} },
        Route { to: "/blog", BlogList {} },
        Route { to: "/blog/welcome", BlogWelcome {} },
        Route { to: "/blog/:post", BlogPost {} },
    }
})
```

### Nested `Routes`

Routes can be composed at various levels, so you don't just need top-level routes. To do this, simple add Routes inside other Routes

```rust
cx.render(rsx! {
    Router {
        Route { to: "/", Component {} },
        Route { to: "/blog",
            BlogContainer {
                h1 { "blog" } // always renders as long as we're on the "blog" subroute
                Route { to: "/", BlogList {} }
                Route { to: "welcome", BlogWelcome {} }
                Route { to: ":post", BlogPost {} }
            }
        },
    }
})
```

### Navigating with `Links`

To navigate your app, regular, old, `a` tags are not going to work. We provide the `Link` component that wraps an `a` tag with the appropriate `href` attribute that generates semantic HTML. You can pass any children into this component and they will become clickable to the appropriate route.

```rust
Link { to: "/blog/welcome",
    h1 { "Welcome to my blog!" }
}
```

#### Active `Links`

When your app has been navigated to a route that matches the route of a `Link`, this `Link` becomes 'active'.
Active links have a special class attached to them. By default it is simply called `"active"` but it can be
modified on the `Link` level or on the `Router` level. Both is done through the prop `active_class`.
If the active class is given on both, the `Router` and the `Link`, the one on the `Link` has precedence.

```rust
Router {
    active_class: "custom-active",  // All active links in this router get this class.
    Link { to: "/", "Home" },
    Link { 
        to: "/blog",
        active_class: "is-active",  // Only for this Link. Overwrites "custom-active" from Router.
        "Blog" 
    },
}
```

### Segments

Each route in your app is comprised of segments and queries. Segments are the portions of the route delimited by forward slashes.

For the route `/dogs/breeds/yorkie/hugo` our "segment list" would be:

```rust
vec!["dogs", "breeds", "yorkie", "hugo"]
```

For any route, you can get a handle the current route with the `use_route` hook.

```rust
fn Title(cx: Scope) -> Element {
    let route = use_route(&cx);

    assert_eq!(route.segments(), &["dogs", "breeds", "yorkie", "hugo"]);

    assert_eq!(route.nth_segment(1), "breeds");

    assert_eq!(route.last_segment(), "hugo");
}
```

As we've shown above, segments can also be named. We can get these named segments out by the last match at that route level:

```rust
// For this router:
Router {
    Route { to: "/", Component {} },
    Route { to: "/blog", BlogList {} },
    Route { to: "/blog/:post", BlogPost {} },
}

fn BlogPost(cx: Scope) -> Element {
    let route = use_route(&cx);

    match route.segment("post").and_then(parse) {
        Some(post) => cx.render(rsx!{ div { "Post {post}" } })
        None => cx.render(rsx!{ div { "Could not find that blog post" } }),
    }
}
```


### Queries




### Listeners

It's possible to connect to route change events from the router by attaching a listener to the Router's `onchange` parameter. This listener is guaranteed to run before any of your routes are matched, so you can perform redirects, add some logging, fetch some data, or do anything that you might want to be synchronous with clicks on Links.

```rust
fn app() {
    cx.render(rsx! {
        Router {
            onchange: move |router| {
                let current = router.current_route();
                log::debug!("App has navigated to {:?}", current);

                // perform a redirect
                if current == "404" {
                    router.navigate_to("/");
                }
            },
            Route { to: "/", Component {} },
        }
    })
}
```


Listeners can also be attached downstream in your app with the `RouteListener` handler component:

```rust
fn TitleCard(cx: Scope) -> Element {
    let (title, set_title) = use_state(&cx, || "First");

    cx.render(rsx!{
        h1 { "render {title}" }

        RouteListener { onchange: move |_| set_title("Last") }
    })
}
```


### Working with Github Pages and other static hosts

Most "static" hosts will have issues with single-page-app (SPA) routers. To get around this, you can either generate an index.html for each route or hijack the 404 page.

For generating a static index.html, see `Generating a Route List`.

To hijack the 404 page, we can simply make a copy of our index.html page and call it 404.html. When Github Pages serves this 404 page, your app will be served instead and the router will render the right corresponding route.

https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-custom-404-page-for-your-github-pages-site

### Generating a SiteMap or Route List

If you want to statically generate and rehydrate all your pages, lean on Dioxus Router to do the heavy lifting.

For this feature to work properly, each route (and nested) route will need to be probed, but this can be done automatically.

```rust
let mut dom = VirtualDom::new(app);
dom.inject_root_context(RouterContext::new());

// populate the router
let _ = dom.rebuild();

// load the router context from the dom, generate a sitemap, and then pre-render each page
let mut prerendered_pages = dom
    .consume_root_context::<RouterContext>()
    .unwrap()
    .sitemap()
    .into_iter()
    .map(|route| {
        // update the root context
        router.navigate_to(route);

        // force our app to update
        let _ = dom.rebuild();

        // render the page and insert it into our map
        (route, dioxus::ssr::render_vdom(&dom))
    })
    .collect::<HashMap<_, _>>();
```
