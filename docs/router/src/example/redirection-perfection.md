# Redirection Perfection

You're well on your way to becoming a routing master!

In this chapter, we will cover creating redirects

## Creating Redirects

A redirect is very simple. When dioxus encounters a redirect while finding out
what components to render, it will redirect the user to the target of the
redirect.

As a simple example, let's say you want user to still land on your blog, even
if they used the path `/myblog` or `/myblog/:name`.

Redirects are special attributes in the router enum that accept a route and a closure
with the route parameters. The closure should return a route to redirect to.

Let's add a redirect to our router enum:

```rust, no_run
{{#include ../../examples/full_example.rs:router}}
```

That's it! Now your users will be redirected to the blog.

### Conclusion

Well done! You've completed the Dioxus Router guide. You've built a small
application and learned about the many things you can do with Dioxus Router.
To continue your journey, you attempt a challenge listed below, look at the [router examples](https://github.com/DioxusLabs/dioxus/tree/master/packages/router/examples), or
can check out the [API reference](https://docs.rs/dioxus-router/).

### Challenges

- Organize your components into separate files for better maintainability.
- Give your app some style if you haven't already.
- Build an about page so your visitors know who you are.
- Add a user system that uses URL parameters.
- Create a simple admin system to create, delete, and edit blogs.
- If you want to go to the max, hook up your application to a rest API and database.
