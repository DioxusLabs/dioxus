# Redirection Perfection
You're well on your way to becoming a routing master!

In this chapter we will cover utilizing the ``Redirect`` component so you can
take Rickrolling to the next level.

### What Is This Redirect Thing?
A redirect is very simple. When dioxus encounters a redirect while finding out
what components to render, it will redirect the user to the target of the
redirect.

As a simple example, let's say you want user to still land on your blog, even
if they used the path `/myblog`.

All we need to do is update our route definition in our app component:
```rust
let routes = cx.use_hook(|_| Segment {
    index: RcComponent(Home),
    fixed: vec![
        (
            String::from("blog"),
            Route {
                content: RcComponent(Blog),
                sub: Some(Segment {
                    index: RcComponent(BlogList),
                    dynamic: DrParameter {
                        name: Some("blog_post"),
                        key: "post_id",
                        content: RcComponent(BlogPost),
                        sub: None,
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
        ),
        // new stuff starts here
        (
            String::from("myblog"),
            Route {
                content: RcRedirect(ItPath(String::from("/blog"))),
                ..Default::default()
            },
        ),
        // new stuff ends here
    ],
    ..Default::default()
});
```

That's it! Now your users will be redirected to the blog.

Notice that instead of `RcComponent` we used `RcRedirect` to tell the router
that this is a redirect.

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
