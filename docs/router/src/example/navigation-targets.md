# Navigation Targets
In the previous chapter we learned how to create links to pages within our app.
We told them where to go using the `target` property. This property takes a
`NavigationTarget`.

## What is a navigation target?
A navigation target is similar to the `href` of an HTML anchor element. It tells
the router where to navigate to. The Dioxus Router knows three kinds of
navigation targets:
- `NtPath`: we already saw that. It's basically an `href`, but cannot link to
  content outside our app.
- `NtExternal`: This works exactly like an HTML anchors `href`. In fact, it is
  just passed through. Don't use this for in-app navigation as it'll trigger a
  page reload by the browser.
- `NtName`: this is the most interesting form of navigation target. We'll look
  at it in detail in this chapter.

## External navigation
If we need a link to an external page we can do it like this:
```rust,ignore
import dioxus::prelude::*;

#[accept(non_snake_case)]
fn GoToDioxus(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NtExternal(String::from("https://dioxuslabs.com")),
            "Go to the dioxus home page"
        }
    })
}
```

## Named navigation
When we previously declared our routes you might have noticed that we set `name`
of our `DrParameter` route to `None`. In fact, we did this to the `/blog` route
as well, but hidden through `..Default::default()`.

These name fields allow us to give our routes optional static names. This allows
us to tell the router to navigate to a specific route by its name instead of its
path.

Let's try that now! First, change the name ouf our `BlogPost` route to
`Some("blog_post")`.

Now we can change the target of the link to the first post to this:
`NtName("blog_post", vec![("post_id", String::from("1"))], QNone)`.
As yo can see, the first value is the routes name. The second value is a vector
containing the parameters that the route needs. The last parameter is for the
query string; `QNone` means no query string.

### The special `root_index` name
Whether we define any names or not, the router always knows about the
`root_index` name. Navigating to it tells the router to go to `/`.

### Use cases for named navigation
- not having to remember whole paths or care about what the current path is
- changing paths later won't break internal links
- paths can easily be localized without affecting navigation

## `InternalNavigationTarget`
In addition to the `NavigationTarget` enum with the three variants described
above, there is an `InternalNavigationTarget`.

It is basically the same as `NavigationTarget`, but lacking the `NtExternal`
variant. It is used for defining redirects (next chapter) and programmatic
navigation.
