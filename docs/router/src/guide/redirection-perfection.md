# Redirection Perfection
You're well on your way to becoming a routing master!

In this chapter we will cover utilizing the ``Redirect`` component so you can take Rickrolling to the next level. We will also provide some optional challenges at the end if you want to continue your practice with not only Dioxus Router but with Dioxus in general.

### What Is This Redirect Thing?
The ``Redirect`` component is simple! When Dioxus determines that it should be rendered, it will redirect your application visitor to wherever you want. 
In this example, let's say that you added a secret page to your site but didn't have time to program in the permission system. As a quick fix you add a redirect.

#### WIP

If you want to route to an external link, just add ``external: true`` to your redirect component.
```rs
Redirect {
    to: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    external: true,
}
```

### Conclusion 
Well done! You've completed the Dioxus Router guide book. You've built a small application and learned about the many things you can do with Dioxus Router. To continue your journey, you can find a list of challenges down below, or you can check out the [reference](../reference/index.md).

### Challenges
- Give your app some style if you haven't already.
- Build an about page so your visitors know who you are.
- Add a user system that uses URL parameters.
- Create a simple admin system to create, delete, and edit blogs.
- If you want to go to the max, hook up your application to a rest API and database.