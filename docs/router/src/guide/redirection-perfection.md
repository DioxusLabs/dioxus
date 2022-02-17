# Redirection Perfection
You're well on your way to becoming a routing master!

In this chapter we will cover utilizing the ``Redirect`` component so you can take Rickrolling to the next level. We will also provide some optional challenges at the end if you want to continue your practice with not only Dioxus Router but with Dioxus in general.

### What Is This Redirect Thing?
The ``Redirect`` component is simple! When Dioxus determines that it should be rendered, it will redirect your application visitor to wherever you want. 
In this example, let's say that you added a secret page to your site but didn't have time to program in the permission system. As a quick fix you add a redirect.

As always, let's first create a new component named ``secret_page``.
```rs
fn secret_page(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "This page is not to be viewed!" }
    })
}
```
To redirect our visitors, all we have to do is render the ``Redirect`` component. The ``Redirect`` component is very similar to the ``Link`` component. The main difference is it doesn't display anything new.
First import the ``Redirect`` component and then update your ``secret_page`` component:
```rs
use dioxus::{
    prelude::*,
    router::{use_route, Link, Redirect, Route, Router}, // UPDATED
};

...

fn secret_page(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "This page is not to be viewed!" }
        Redirect { to: "/" } // NEW
    })
}
```
That's it! Now your users will be redirected away from the secret page.

>Similar to the ``Link`` component, the ``Redirect`` component needs to be explicitly set to redirect to an external site. To link to external sites, add the ``external: true`` property.
>```rs 
>Redirect { to: "https://github.com", external: true}
>```

### Conclusion 
Well done! You've completed the Dioxus Router guide book. You've built a small application and learned about the many things you can do with Dioxus Router. To continue your journey, you can find a list of challenges down below, or you can check out the [reference](../reference/index.md).

### Challenges
- Organize your components into seperate files for better maintainability.
- Give your app some style if you haven't already.
- Build an about page so your visitors know who you are.
- Add a user system that uses URL parameters.
- Create a simple admin system to create, delete, and edit blogs.
- If you want to go to the max, hook up your application to a rest API and database.