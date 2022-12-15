# Redirection Perfection
You're well on your way to becoming a routing master!

In this chapter we will cover utilizing redirects so you can take Rickrolling to
the next level.

## What Is This Redirect Thing?
A redirect is very simple. When dioxus encounters a redirect while finding out
what components to render, it will redirect the user to the target of the
redirect.

As a simple example, let's say you want user to still land on your blog, even
if they used the path `/myblog`.

All we need to do is update our route definition in our app component:
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
# fn App(cx: Scope) -> Element {
let routes = use_segment(&cx, || {
    Segment::new()
        .index(Home as Component)
        .fixed(
            "blog",
            Route::new(Blog as Component).nested(
                Segment::new().index(BlogList as Component).catch_all(
                    ParameterRoute::new("post_id", BlogPost as Component).name(BlogPost)
                ),
            ),
        )
        .fixed("myblog", "/blog") // this is new
        .fallback(PageNotFound as Component)
});
# unimplemented!()
# }
```

That's it! Now your users will be redirected to the blog.

Notice that the `"/blog"` `str` is a [navigation target](./navigation-targets.md).
We could also use external or named targets.

### Conclusion
Well done! You've completed the Dioxus Router guide book. You've built a small
application and learned about the many things you can do with Dioxus Router.
To continue your journey, you can find a list of challenges down below, or you
can check out the [API reference](https://docs.rs/dioxus-router/).

### Challenges
- Organize your components into seperate files for better maintainability.
- Give your app some style if you haven't already.
- Build an about page so your visitors know who you are.
- Add a user system that uses URL parameters.
- Create a simple admin system to create, delete, and edit blogs.
- If you want to go to the max, hook up your application to a rest API and database.
