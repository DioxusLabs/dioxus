# "Hello, World" desktop app



If you plan to develop extensions for the `Dioxus` ecosystem, please use the `dioxus` crate with the `core` feature to limit the amount of dependencies your project brings in.


### What is this `Scope` object?

Coming from React, the `Scope` object might be confusing. In React, you'll want to store data between renders with hooks. However, hooks rely on global variables which make them difficult to integrate in multi-tenant systems like server-rendering.

In Dioxus, you are given an explicit `Scope` object to control how the component renders and stores data. The `Scope` object provides a handful of useful APIs for features like suspense, rendering, and more.

For now, just know that `Scope` lets you store state with hooks and render elements with `cx.render`.
