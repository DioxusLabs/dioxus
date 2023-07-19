# Adding the Router to Your Application

In this chapter, we will learn how to add the router to our app. By itself, this
is not very useful. However, it is a prerequisite for all the functionality
described in the other chapters.

> Make sure you added the `dioxus-router` dependency as explained in the
> [introduction](../index.md).

In most cases, we want to add the router to the root component of our app. This
way, we can ensure that we have access to all its functionality everywhere.

First, we define the router with the router macro:

```rust, no_run
{{#include ../../examples/first_route.rs:router}}
```

Then we render the router with the [`Router`] component.

```rust, no_run
{{#include ../../examples/first_route.rs:app}}
```
