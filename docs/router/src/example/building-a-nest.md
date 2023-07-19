# Building a Nest

In this chapter, we will begin to build the blog portion of our site which will
include links, nested routes, and route parameters.

## Site Navigation

Our site visitors won't know all the available pages and blogs on our site so we
should provide a navigation bar for them. Our navbar will be a list of links going between our pages.

We want our navbar component to be rendered on several different pages on our site. Instead of duplicating the code, we can create a component that wraps all children routes. This is called a layout component. To tell the router where to render the child routes, we use the [`Outlet`] component.

Let's create a new `NavBar` component:

```rust, no_run
{{#include ../../examples/nested_routes.rs:nav}}
```

Next, let's add our `NavBar` component as a layout to our Route enum:

```rust, no_run
{{#include ../../examples/nested_routes.rs:router}}
```

To add links to our `NavBar`, we could always use an HTML anchor element but that has two issues:

1. It causes a full-page reload
2. We can accidentally link to a page that doesn't exist

Instead, we want to use the [`Link`] component provided by Dioxus Router.

The [`Link`] is similar to a regular `<a>` tag. It takes a target and children.

Unlike a regular `<a>` tag, we can pass in our Route enum as the target. Because we annotated our routes with the [`route(path)`] attribute, the [`Link`] will know how to generate the correct URL. If we use the Route enum, the rust compiler will prevent us from linking to a page that doesn't exist.

Let's add our links:

```rust, no_run
{{#include ../../examples/links.rs:nav}}
```

> Using this method, the [`Link`] component only works for links within our
> application. To learn more about navigation targets see
> [here](./navigation-targets.md).

Now you should see a list of links near the top of your page. Click on one and
you should seamlessly travel between pages.

## URL Parameters and Nested Routes

Many websites such as GitHub put parameters in their URL. For example,
`https://github.com/DioxusLabs` utilizes the text after the domain to
dynamically search and display content about an organization.

We want to store our blogs in a database and load them as needed. We also
want our users to be able to send people a link to a specific blog post.
Instead of listing all of the blog titles at compile time, we can make a dynamic route.

We could utilize a search page that loads a blog when clicked but then our users
won't be able to share our blogs easily. This is where URL parameters come in.

The path to our blog will look like `/blog/myBlogPage`, `myBlogPage` being the
URL parameter.

First, let's create a layout component (similar to the navbar) that wraps the blog content. This allows us to add a heading that tells the user they are on the blog.

```rust, no_run
{{#include ../../examples/dynamic_route.rs:blog}}
```

Now we'll create another index component, that'll be displayed when no blog post
is selected:

```rust, no_run
{{#include ../../examples/dynamic_route.rs:blog_list}}
```

We also need to create a component that displays an actual blog post. This component will accept the URL parameters as props:

```rust, no_run
{{#include ../../examples/dynamic_route.rs:blog_post}}
```

Finally, let's tell our router about those components:

```rust, no_run
{{#include ../../examples/dynamic_route.rs:router}}
```

That's it! If you head to `/blog/1` you should see our sample post.

## Conclusion

In this chapter, we utilized Dioxus Router's Link, and Route Parameter
functionality to build the blog portion of our application. In the next chapter,
we will go over how navigation targets (like the one we passed to our links)
work.

[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/prelude/fn.GenericLink<R>.html
