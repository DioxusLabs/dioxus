# Building a Nest
Not a bird's nest! A nest of routes!

In this chapter we will begin to build the blog portion of our site which will include links, nested URLs, and URL parameters. We will also explore the use case of rendering components outside of routes.

### Site Navigation
Our site visitors won't know all the available pages and blogs on our site so we should provide a navigation bar for them.
Let's create a new ``navbar`` component:
```rs
fn navbar(cx: Scope) -> Element {
    cx.render(rsx! {
        ul {
            
        }
    })
}
```
Our navbar will be a list of links going between our pages. We could always use an HTML anchor element but that would cause our page to unnecessarily reload. Instead we want to use the ``Link`` component provided by Dioxus Router. 

The Link component is very similar to the Route component. It takes a path and an element. Add the Link component into your use statement and then add some links:
```rs
use dioxus::{
    prelude::*,
    router::{Route, Router, Link}, // UPDATED
};

...

fn navbar(cx: Scope) -> Element {
    cx.render(rsx! {
        ul {
            // NEW
            Link { to: "/", "Home"}
            br {}
            Link { to: "/blog", "Blog"}
        }
    })
}
```
>By default, the Link component only works for links within your application. To link to external sites, add the ``external: true`` property.
>```rs 
>Link { to: "https://github.com", external: true, "GitHub"}
>```

And finally, use the navbar component in your app component:
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            self::navbar {} // NEW
            Route { to: "/", self::homepage {}}
            Route { to: "", self::page_not_found {}}
        }
    })
}
```
Now you should see a list of links near the top of your page. Click on one and you should seamlessly travel between pages.

##### WIP: Active Link Styling

### URL Parameters and Nested Routes
Many websites such as GitHub put parameters in their URL. For example, ``github.com/DioxusLabs`` utilizes the text after the domain to dynamically search and display content about an organization.

We want to store our blogs in a database and load them as needed. This'll help prevent our app from being bloated therefor providing faster load times. We also want our users to be able to send people a link to a specific blog post.
We could utilize a search page that loads a blog when clicked but then our users won't be able to share our blogs easily. This is where URL parameters come in. And finally, we also want our site to tell users they are on a blog page whenever the URL starts with``/blog``.

The path to our blog will look like ``/blog/myBlogPage``. ``myBlogPage`` being the URL parameter.
Dioxus Router uses the ``:name`` pattern so our route will look like ``/blog/:post``.  

First, lets tell users when they are on a blog page. Add a new route in your app component.
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            self::navbar {}
            Route { to: "/", self::homepage {}}
            // NEW
            Route { 
                to: "/blog",
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```
Routes can take components as parameters and we know that a route is a component. We nest routes by doing exactly what they are called, nesting them:
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            self::navbar {}
            Route { to: "/", self::homepage {}}
            Route { 
                to: "/blog",
                Route { to: "/:post", "This is my blog post!" } // NEW
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```
Nesting our route like this isn't too helpful at first, but remember we want to tell users they are on a blog page. Let's move our ``p { "-- Dioxus Blog --" }`` inside of our ``/blog`` route.
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            self::navbar {}
            Route { to: "/", self::homepage {}}
            Route { 
                to: "/blog",
                p { "-- Dioxus Blog --" } // MOVED
                Route { to: "/:post", "This is my blog post!" }
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```
Now our ``-- Dioxus Blog --`` text will be displayed whenever a user is on a path that starts with ``/blog``. Displaying content in a way that is page-agnostic is useful when building navigation menus, footers, and similar. 

All that's left is to handle our URL parameter. We will begin by creating a ``get_blog_post`` function. In a real site, this function would call an API endpoint to get a blog post from the database. However, that is out of the scope of this guide so we will be utilizing static text.
```rs
fn get_blog_post(id: &str) -> String {
    match id {
        "foo" => "Welcome to the foo blog post!".to_string(),
        "bar" => "This is the bar blog post!".to_string(),
        id => format!("Blog post '{id}' does not exist!")
    }
}

```
Now that we have established our helper function, lets create a new ``blog_post`` component.
```rs
fn blog_post(cx: Scope) -> Element {
    let blog_text = "";

    cx.render(rsx! {
        p { "{blog_text}" }
    })
}
```
All that's left is to extract the blog id from the URL and to call our helper function to get the blog text. To do this we need to utilize Dioxus Router's ``use_route`` hook.
First start by adding ``use_route`` to your imports and then utilize the hook in your ``blog_post`` component.
```rs
use dioxus::{
    prelude::*,
    router::{use_route, Link, Route, Router}, // UPDATED
};

...

fn blog_post(cx: Scope) -> Element {
    let route = use_route(&cx); // NEW
    let blog_text = "";

    cx.render(rsx! {
        p { "{blog_text}" }
    })
}
```
Dioxus Router provides built in methods to extract information from a route. We could utilize the ``segments``, ``nth_segment``, or ``last_segment`` method for our case but we'll use the ``segment`` method which extracts a specific URL parameter.
The ``segment`` method also parses the parameter into any type for us. We'll use a match expression that handles a parsing error and on success, uses our helper function to grab the blog post.
```rs
fn blog_post(cx: Scope) -> Element {
    let route = use_route(&cx);

    // NEW
    let blog_text = match route.segment::<String>("post").unwrap() {
        Ok(val) => get_blog_post(&val),
        Err(_) => "An unknown error occured".to_string(),
    };

    cx.render(rsx! {
        p { "{blog_text}" }
    })
}
```
And finally add the ``blog_post`` component to your ``app`` component:
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            self::navbar {}
            Route { to: "/", self::homepage {}}
            Route {
                to: "/blog",
                p { "-- Dioxus Blog --" }
                Route { to: "/:post", self::blog_post {} } // UPDATED
            }
            Route { to: "", self::page_not_found {}}
        }
    })
}
```
That's it! If you head to ``/blog/foo`` you should see ``Welcome to the foo blog post!``.

### Conclusion
In this chapter we utilized Dioxus Router's Link, URL Parameter, and ``use_route`` functionality to build the blog portion of our application. In the next and final chapter, we will go over the ``Redirect`` component to redirect non-authorized users to another page.